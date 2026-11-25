//! Tự tìm máy giáo viên và tự nối về.
//!
//! Agent gửi gói "seek" (UDP 47901, quảng bá trong mạng LAN và tới các địa chỉ cố định nếu có).
//! Máy giáo viên trả lời "here" kèm mã giáo viên và cổng. Agent nối TLS tới máy giáo viên đó:
//!  - nếu mã giáo viên đã được ghép trước: kiểm tra chữ ký là xong;
//!  - nếu chưa: ghép bằng **mật khẩu phòng** (SPAKE2), rồi lưu mã giáo viên cho các lần sau.
//!
//! Máy học sinh không mở cổng nào, nên không cần mở tường lửa trên máy học sinh.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use lc_core::proto::{self, Ctrl, Discover, K_CTRL};
use lc_core::{auth, tls, DISCOVER_PORT, PROTO_VERSION, TEACHER_PORT};
use serde_json::json;
use tokio::net::UdpSocket;

use crate::config::PublicConfig;
use crate::helper;
use crate::session::{self, Shared};

async fn public_config() -> PublicConfig {
    match helper::machine(json!({"op": "info"})).await {
        Ok(v) => serde_json::from_value(v["config"].clone()).unwrap_or_default(),
        Err(e) => {
            tracing::warn!("Không lấy được cấu hình máy: {e:#}");
            PublicConfig::default()
        }
    }
}

pub async fn run(sh: Arc<Shared>) -> Result<()> {
    let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    sock.set_broadcast(true)?;
    let active: Arc<Mutex<HashSet<SocketAddr>>> = Default::default();
    // máy giáo viên vừa từ chối (sai mật khẩu...) thì chờ một lúc mới thử lại
    let backoff: Arc<Mutex<HashMap<SocketAddr, Instant>>> = Default::default();

    // gửi "seek" định kỳ
    {
        let sock = sock.clone();
        let active = active.clone();
        tokio::spawn(async move {
            loop {
                let cfg = public_config().await;
                *sh_cfg().lock().unwrap() = cfg.clone();
                let msg = serde_json::to_vec(&Discover { lc: PROTO_VERSION, kind: "seek".into(), room: cfg.room.clone(),
                                                         port: 0, teacher: String::new(), name: String::new() }).unwrap();
                let mut targets: Vec<String> = vec![format!("255.255.255.255:{DISCOVER_PORT}")];
                if let Ok(t) = std::env::var("LABCONTROL_SEEK_TO") {
                    targets = t.split(',').map(|s| s.trim().to_string()).collect();
                }
                for h in &cfg.teacher_hosts {
                    let h = h.trim();
                    if !h.is_empty() {
                        targets.push(if h.contains(':') { h.to_string() } else { format!("{h}:{DISCOVER_PORT}") });
                    }
                }
                for t in &targets {
                    let _ = sock.send_to(&msg, t).await;
                }
                // đã nối thì hỏi thưa hơn
                let wait = if active.lock().unwrap().is_empty() { 3 } else { 10 };
                tokio::time::sleep(Duration::from_secs(wait)).await;
            }
        });
    }

    let mut buf = vec![0u8; 2048];
    loop {
        let (n, from) = sock.recv_from(&mut buf).await?;
        let Ok(d) = serde_json::from_slice::<Discover>(&buf[..n]) else { continue };
        if d.kind != "here" || d.lc != PROTO_VERSION {
            continue;
        }
        let cfg = sh_cfg().lock().unwrap().clone();
        if !cfg.room.is_empty() && !d.room.is_empty() && !cfg.room.eq_ignore_ascii_case(&d.room) {
            continue;
        }
        let known = cfg.teachers.iter().any(|t| t == &d.teacher);
        if !known && !cfg.has_password {
            continue; // chưa ghép và không có mật khẩu phòng: không nối
        }
        let addr = SocketAddr::new(from.ip(), if d.port == 0 { TEACHER_PORT } else { d.port });
        if active.lock().unwrap().contains(&addr) {
            continue;
        }
        if let Some(t) = backoff.lock().unwrap().get(&addr) {
            if t.elapsed() < Duration::from_secs(30) {
                continue;
            }
        }
        active.lock().unwrap().insert(addr);
        let (sh, active, backoff) = (sh.clone(), active.clone(), backoff.clone());
        tokio::spawn(async move {
            tracing::info!("Nối tới máy giáo viên {} ({addr})", d.name);
            match connect(addr, &sh).await {
                Ok(_) => tracing::info!("Máy giáo viên {addr} đã ngắt"),
                Err(e) => {
                    tracing::info!("Kết nối {addr}: {e:#}");
                    backoff.lock().unwrap().insert(addr, Instant::now());
                }
            }
            active.lock().unwrap().remove(&addr);
        });
    }
}

fn sh_cfg() -> &'static Mutex<PublicConfig> {
    static C: std::sync::OnceLock<Mutex<PublicConfig>> = std::sync::OnceLock::new();
    C.get_or_init(Default::default)
}

async fn connect(addr: SocketAddr, sh: &Arc<Shared>) -> Result<()> {
    let tcp = tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(addr)).await
        .map_err(|_| anyhow!("hết thời gian chờ"))??;
    tcp.set_nodelay(true)?;
    let (cc, seen) = tls::client_config();
    let name = tokio_rustls::rustls::pki_types::ServerName::try_from("labcontrol-teacher")?.to_owned();
    let s = tokio::time::timeout(Duration::from_secs(10), tokio_rustls::TlsConnector::from(cc).connect(name, tcp)).await??;
    let fp = seen.lock().unwrap().clone().ok_or_else(|| anyhow!("không có chứng chỉ"))?;
    let (mut rd, mut wr) = tokio::io::split(s);

    // ----- 1. máy giáo viên chào
    let (teacher, teacher_name) = match next(&mut rd).await? {
        Ctrl::TeacherHello { proto: p, teacher, teacher_name, .. } if p == PROTO_VERSION => (teacher, teacher_name),
        Ctrl::TeacherHello { proto: p, .. } => bail!("máy giáo viên dùng giao thức {p}, máy này dùng {PROTO_VERSION}"),
        _ => bail!("máy giáo viên trả lời lạ"),
    };
    auth::parse_teacher_code(&teacher)?;
    let cfg = public_config().await;
    let known = cfg.teachers.iter().any(|t| t == &teacher);
    let mode = if known { "key" } else if cfg.has_password { "pair" } else { "none" };

    // ----- 2. Agent giới thiệu
    let nonce = auth::random_nonce();
    proto::write_ctrl(&mut wr, &Ctrl::AgentHello { info: sh.info(&cfg), nonce: nonce.clone(), mode: mode.into() }).await?;

    // ----- 3. máy giáo viên ký nonce + dấu vân tay TLS (chứng minh giữ khóa riêng của mã giáo viên)
    match next(&mut rd).await? {
        Ctrl::Auth { sig } if auth::verify(std::slice::from_ref(&teacher), &teacher, &sig, &nonce, &fp) => {}
        Ctrl::AuthFail { reason } => bail!("máy giáo viên từ chối: {reason}"),
        _ => {
            let _ = proto::write_ctrl(&mut wr, &Ctrl::AuthFail { reason: "chữ ký không hợp lệ".into() }).await;
            bail!("chữ ký máy giáo viên không hợp lệ");
        }
    }

    match mode {
        "key" => {}
        "pair" => {
            // ----- 4. ghép bằng mật khẩu phòng (phần mật mã do Helper làm, Agent không thấy mật khẩu)
            let st = helper::machine(json!({"op": "pair_start"})).await?;
            let pid = st["id"].as_str().unwrap_or("").to_string();
            proto::write_ctrl(&mut wr, &Ctrl::PairStart { msg: st["msg"].as_str().unwrap_or("").into() }).await?;
            let (peer, tconf) = match next(&mut rd).await? {
                Ctrl::PairReply { msg, confirm } => (msg, confirm),
                Ctrl::AuthFail { reason } => bail!("máy giáo viên từ chối: {reason}"),
                _ => bail!("máy giáo viên trả lời lạ"),
            };
            let fin = helper::machine(json!({"op": "pair_finish", "id": pid, "peer": peer, "fp": fp,
                                             "teacher": teacher, "confirm": tconf})).await;
            match fin {
                Ok(v) => proto::write_ctrl(&mut wr, &Ctrl::PairConfirm { confirm: v["confirm"].as_str().unwrap_or("").into() }).await?,
                Err(e) => {
                    let _ = proto::write_ctrl(&mut wr, &Ctrl::AuthFail { reason: format!("{e:#}") }).await;
                    bail!("ghép máy thất bại: {e:#}");
                }
            }
            tracing::info!("Đã ghép với máy giáo viên '{teacher_name}' bằng mật khẩu phòng");
        }
        _ => {
            let _ = proto::write_ctrl(&mut wr, &Ctrl::AuthFail { reason: "máy chưa có mật khẩu phòng".into() }).await;
            bail!("máy chưa được ghép");
        }
    }

    // ----- 5. máy giáo viên xác nhận
    match next(&mut rd).await? {
        Ctrl::AuthOk => {}
        Ctrl::AuthFail { reason } => bail!("máy giáo viên từ chối: {reason}"),
        _ => bail!("máy giáo viên trả lời lạ"),
    }
    tracing::info!("Đã nối với máy giáo viên '{teacher_name}'");
    if let Err(e) = session::run(rd, wr, sh.clone(), teacher).await {
        tracing::info!("Phiên với '{teacher_name}' kết thúc: {e:#}");
    }
    Ok(()) // mất kết nối sau khi đã vào lớp thì nối lại ngay, không chờ
}

async fn next<R: tokio::io::AsyncRead + Unpin>(rd: &mut R) -> Result<Ctrl> {
    let (k, d) = tokio::time::timeout(Duration::from_secs(20), proto::read_frame(rd)).await
        .map_err(|_| anyhow!("máy giáo viên không trả lời"))??;
    if k != K_CTRL {
        bail!("gói tin lạ khi bắt tay");
    }
    proto::parse_ctrl(&d)
}
