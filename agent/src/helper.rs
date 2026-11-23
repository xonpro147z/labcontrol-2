//! Helper: dịch vụ quyền cao (Windows Service chạy SYSTEM / systemd chạy root) trên máy học sinh.
//!
//! Việc của Helper:
//!  1. Giữ mật khẩu phòng (học sinh không đọc được) và thực hiện phần mật mã của bước ghép máy.
//!  2. Lưu mã giáo viên đã ghép.
//!  3. Chặn trang web (sửa file hosts) và đổi mật khẩu phòng, CHỈ khi lệnh có chữ ký của giáo viên đã ghép.
//!
//! Agent (chạy dưới tài khoản học sinh) nói chuyện với Helper qua 127.0.0.1:47903, mỗi dòng một JSON.
//! Học sinh cũng gọi được cổng này, nhưng không có khóa riêng của giáo viên nên không ra lệnh được;
//! thử đoán mật khẩu bị giới hạn 1 lần/giây và khóa 5 phút sau 10 lần sai.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use lc_core::{auth, pair, proto::Policy, HELPER_PORT};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::config;

const HOSTS_BEGIN: &str = "# --- LabControl BEGIN (do LabControl quản lý, đừng sửa tay) ---";
const HOSTS_END: &str = "# --- LabControl END ---";

#[derive(Default)]
struct Guard {
    fails: Vec<Instant>,
    last: Option<Instant>,
}

#[derive(Default)]
struct State {
    pairings: HashMap<String, (pair::Pairing, Instant)>,
    guard: Guard,
    last_policy_ts: u64,
    last_pw_ts: u64,
    /// Lần cuối giáo viên gửi lại chính sách chặn web. Máy giáo viên gửi lại mỗi phút;
    /// quá 3 phút không nhận được (giáo viên tắt Console, mất mạng) thì tự bỏ chặn.
    policy_at: Option<Instant>,
    last_update_ts: u64,
    /// Đang chạy như dịch vụ riêng (không phải chạy ké trong Agent khi máy chưa cài Helper)
    service: bool,
}

const POLICY_TTL: Duration = Duration::from_secs(180);

fn expiry_loop(st: Arc<Mutex<State>>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(10));
        let expired = {
            let mut s = st.lock().unwrap();
            match s.policy_at {
                Some(t) if t.elapsed() > POLICY_TTL => {
                    s.policy_at = None;
                    true
                }
                _ => false,
            }
        };
        if expired {
            tracing::info!("Hết hạn chính sách chặn web, bỏ chặn");
            let _ = apply_hosts(&[]);
        }
    });
}

pub async fn serve() -> Result<()> {
    let l = tokio::net::TcpListener::bind(("127.0.0.1", HELPER_PORT)).await
        .map_err(|e| anyhow!("Helper không mở được cổng {HELPER_PORT}: {e}"))?;
    tracing::info!("Helper sẵn sàng ở 127.0.0.1:{HELPER_PORT}");
    // khởi động lại thì bỏ chặn cũ; giáo viên đang dạy sẽ gửi lại trong vòng 1 phút
    let _ = apply_hosts(&[]);
    let st = Arc::new(Mutex::new(State { service: true, ..Default::default() }));
    expiry_loop(st.clone());
    loop {
        let (s, _) = l.accept().await?;
        let st = st.clone();
        tokio::spawn(async move {
            let (r, mut w) = s.into_split();
            let mut lines = BufReader::new(r).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let reply = match serde_json::from_str::<Value>(&line) {
                    Ok(v) => handle(&st, v).unwrap_or_else(|e| json!({"ok": false, "error": format!("{e:#}")})),
                    Err(_) => json!({"ok": false, "error": "yêu cầu không hợp lệ"}),
                };
                if w.write_all(format!("{reply}\n").as_bytes()).await.is_err() {
                    break;
                }
            }
        });
    }
}

fn handle(st: &Arc<Mutex<State>>, v: Value) -> Result<Value> {
    let op = v["op"].as_str().unwrap_or("");
    match op {
        "info" => {
            let c = config::load().unwrap_or_default();
            Ok(json!({"ok": true, "config": c.public()}))
        }
        "pair_start" => {
            let c = config::load()?;
            if c.password.is_empty() {
                bail!("máy này chưa có mật khẩu phòng");
            }
            let mut s = st.lock().unwrap();
            // giới hạn tốc độ thử mật khẩu
            let now = Instant::now();
            s.guard.fails.retain(|t| now.duration_since(*t) < Duration::from_secs(300));
            if s.guard.fails.len() >= 10 {
                bail!("thử sai quá nhiều lần, chờ 5 phút");
            }
            if let Some(l) = s.guard.last {
                if now.duration_since(l) < Duration::from_millis(1000) {
                    bail!("thử quá nhanh");
                }
            }
            s.guard.last = Some(now);
            s.pairings.retain(|_, (_, t)| now.duration_since(*t) < Duration::from_secs(60));
            let (p, msg) = pair::start(&c.password);
            let id = auth::random_nonce();
            s.pairings.insert(id.clone(), (p, now));
            Ok(json!({"ok": true, "id": id, "msg": msg}))
        }
        "pair_finish" => {
            let id = v["id"].as_str().unwrap_or("");
            let (peer, fp, teacher, tconf) = (v["peer"].as_str().unwrap_or(""), v["fp"].as_str().unwrap_or(""),
                                              v["teacher"].as_str().unwrap_or(""), v["confirm"].as_str().unwrap_or(""));
            auth::parse_teacher_code(teacher)?;
            let p = st.lock().unwrap().pairings.remove(id).map(|(p, _)| p).ok_or_else(|| anyhow!("phiên ghép hết hạn"))?;
            let key = p.finish(peer)?;
            if !pair::check(&key, "teacher", fp, teacher, tconf) {
                st.lock().unwrap().guard.fails.push(Instant::now());
                bail!("sai mật khẩu phòng");
            }
            let mut c = config::load()?;
            if !c.teachers.iter().any(|t| t == teacher) {
                c.teachers.push(teacher.to_string());
                config::save(&c)?;
            }
            Ok(json!({"ok": true, "confirm": pair::confirm(&key, "agent", fp, teacher)}))
        }
        "policy" => {
            let pj = v["policy"].as_str().unwrap_or("");
            let ts = v["ts"].as_u64().unwrap_or(0);
            let sig = v["sig"].as_str().unwrap_or("");
            let c = config::load()?;
            if !auth::verify_policy(&c.teachers, pj, ts, sig) {
                bail!("chữ ký không hợp lệ");
            }
            {
                let mut s = st.lock().unwrap();
                if ts < s.last_policy_ts || ts == 0 {
                    bail!("lệnh cũ");
                }
                s.last_policy_ts = ts;
            }
            let p: Policy = serde_json::from_str(pj)?;
            apply_hosts(&p.block_web)?;
            st.lock().unwrap().policy_at = if p.block_web.is_empty() { None } else { Some(Instant::now()) };
            Ok(json!({"ok": true}))
        }
        "set_password" => {
            let pw = v["password"].as_str().unwrap_or("");
            let ts = v["ts"].as_u64().unwrap_or(0);
            let sig = v["sig"].as_str().unwrap_or("");
            let mut c = config::load()?;
            if !auth::verify_policy(&c.teachers, &format!("password:{pw}"), ts, sig) {
                bail!("chữ ký không hợp lệ");
            }
            {
                let mut s = st.lock().unwrap();
                if ts <= s.last_pw_ts {
                    bail!("lệnh cũ");
                }
                s.last_pw_ts = ts;
            }
            c.password = pair::normalize(pw);
            config::save(&c)?;
            Ok(json!({"ok": true}))
        }
        "update" => {
            // Cập nhật Agent do máy giáo viên đẩy xuống. Chỉ nhận khi có chữ ký của giáo viên đã ghép
            // trên đúng mã băm của file, nên học sinh không cài được chương trình lạ qua đây.
            let platform = v["platform"].as_str().unwrap_or("");
            let version = v["version"].as_str().unwrap_or("");
            let sha = v["sha256"].as_str().unwrap_or("");
            let ts = v["ts"].as_u64().unwrap_or(0);
            let sig = v["sig"].as_str().unwrap_or("");
            if platform != std::env::consts::OS {
                bail!("gói cập nhật dành cho {platform}, máy này là {}", std::env::consts::OS);
            }
            let c = config::load()?;
            if !auth::verify_policy(&c.teachers, &auth::update_message(platform, version, sha), ts, sig) {
                bail!("chữ ký gói cập nhật không hợp lệ");
            }
            {
                let mut s = st.lock().unwrap();
                if ts <= s.last_update_ts {
                    bail!("lệnh cũ");
                }
                s.last_update_ts = ts;
            }
            use base64::Engine;
            let data = base64::engine::general_purpose::STANDARD.decode(v["data"].as_str().unwrap_or(""))
                .map_err(|_| anyhow!("dữ liệu cập nhật hỏng"))?;
            if auth::sha256_hex(&data) != sha {
                bail!("file cập nhật bị hỏng khi truyền");
            }
            std::fs::create_dir_all(config::dir())?;
            let staged = config::dir().join("update.bin");
            std::fs::write(&staged, &data)?;
            config::restrict(&staged);
            let r = config::install_binary(&staged);
            let _ = std::fs::remove_file(&staged);
            let dest = r?;
            tracing::info!("Đã cài bản cập nhật {version} vào {}", dest.display());
            if st.lock().unwrap().service {
                // thoát để Windows Service / systemd chạy lại Helper bằng bản mới
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(1500));
                    std::process::exit(3);
                });
            }
            Ok(json!({"ok": true, "path": dest.display().to_string()}))
        }
        _ => bail!("lệnh không hỗ trợ"),
    }
}

fn hosts_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("LABCONTROL_HOSTS") {
        return p.into();
    }
    #[cfg(windows)]
    {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        std::path::PathBuf::from(root).join(r"System32\drivers\etc\hosts")
    }
    #[cfg(not(windows))]
    {
        "/etc/hosts".into()
    }
}

/// Ghi danh sách chặn vào file hosts (giữ nguyên phần khác của file).
pub fn apply_hosts(domains: &[String]) -> Result<()> {
    let p = hosts_path();
    let old = std::fs::read_to_string(&p).unwrap_or_default();
    let mut out = String::new();
    let mut skip = false;
    for line in old.lines() {
        if line.trim() == HOSTS_BEGIN {
            skip = true;
            continue;
        }
        if line.trim() == HOSTS_END {
            skip = false;
            continue;
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    let clean: Vec<String> = domains.iter()
        .map(|d| d.trim().trim_start_matches("https://").trim_start_matches("http://").split('/').next().unwrap_or("").to_lowercase())
        .filter(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'))
        .collect();
    if !clean.is_empty() {
        out.push_str(HOSTS_BEGIN);
        out.push('\n');
        for d in &clean {
            let base = d.trim_start_matches("www.");
            for h in [base.to_string(), format!("www.{base}"), format!("m.{base}")] {
                out.push_str(&format!("0.0.0.0 {h}\n::0 {h}\n"));
            }
        }
        out.push_str(HOSTS_END);
        out.push('\n');
    }
    std::fs::write(&p, out)?;
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("ipconfig").arg("/flushdns").output();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("resolvectl").arg("flush-caches").output();
    }
    Ok(())
}

// ------------------------------------------------------------------ phía Agent: gọi Helper
/// Gọi Helper; nếu máy không cài Helper mà Agent đọc được cấu hình (chạy thử, hoặc máy chỉ có một
/// tài khoản quản trị) thì Agent tự làm phần việc của Helper ngay trong tiến trình.
pub async fn machine(req: Value) -> Result<Value> {
    machine_wait(req, Duration::from_secs(5)).await
}

pub async fn machine_wait(req: Value, wait: Duration) -> Result<Value> {
    match call_wait(req.clone(), wait).await {
        Ok(v) => Ok(v),
        Err(e) if e.downcast_ref::<std::io::Error>().is_some() || e.to_string().contains("không phản hồi") => {
            static LOCAL: std::sync::OnceLock<Arc<Mutex<State>>> = std::sync::OnceLock::new();
            config::load().map_err(|_| anyhow!("Chưa cài Helper và không đọc được cấu hình máy"))?;
            let st = LOCAL.get_or_init(|| {
                let st = Arc::new(Mutex::new(State::default()));
                expiry_loop(st.clone());
                st
            });
            let v = handle(st, req)?;
            Ok(v)
        }
        Err(e) => Err(e),
    }
}

pub async fn has_helper() -> bool {
    call(json!({"op": "info"})).await.is_ok()
}

pub async fn call(req: Value) -> Result<Value> {
    call_wait(req, Duration::from_secs(5)).await
}

async fn call_wait(req: Value, wait: Duration) -> Result<Value> {
    let s = tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(("127.0.0.1", HELPER_PORT))).await
        .map_err(|_| anyhow!("Helper không phản hồi"))??;
    let (r, mut w) = s.into_split();
    w.write_all(format!("{req}\n").as_bytes()).await?;
    let mut lines = BufReader::new(r).lines();
    let line = tokio::time::timeout(wait, lines.next_line()).await
        .map_err(|_| anyhow!("Helper không phản hồi"))??.ok_or_else(|| anyhow!("Helper đóng kết nối"))?;
    let v: Value = serde_json::from_str(&line)?;
    if v["ok"].as_bool() != Some(true) {
        bail!("{}", v["error"].as_str().unwrap_or("Helper báo lỗi"));
    }
    Ok(v)
}

// ------------------------------------------------------------------ Windows Service
#[cfg(windows)]
pub mod service {
    use std::ffi::OsString;
    use std::time::Duration;

    use anyhow::{Context, Result};
    use windows_service::service::{
        ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept, ServiceErrorControl,
        ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod, ServiceInfo, ServiceStartType, ServiceState,
        ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
    use windows_service::{define_windows_service, service_dispatcher};

    pub const NAME: &str = "LabControlHelper";

    pub fn install(exe: &std::path::Path) -> Result<()> {
        let mgr = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)
            .context("Cần chạy bằng quyền quản trị")?;
        let info = ServiceInfo {
            name: OsString::from(NAME),
            display_name: OsString::from("LabControl Helper"),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe.to_path_buf(),
            launch_arguments: vec![OsString::from("helper-service")],
            dependencies: vec![],
            account_name: None,
            account_password: None,
        };
        let svc = match mgr.open_service(NAME, ServiceAccess::all()) {
            Ok(s) => {
                let _ = s.stop();
                std::thread::sleep(Duration::from_secs(1));
                s.change_config(&info)?;
                s
            }
            Err(_) => mgr.create_service(&info, ServiceAccess::all())?,
        };
        let _ = svc.set_description("LabControl: chặn web, giữ mật khẩu phòng cho máy học sinh");
        let _ = svc.update_failure_actions(ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86400)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![ServiceAction { action_type: ServiceActionType::Restart, delay: Duration::from_secs(3) }; 3]),
        });
        let _ = svc.start::<&str>(&[]);
        Ok(())
    }

    pub fn uninstall() -> Result<()> {
        let mgr = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
        let svc = mgr.open_service(NAME, ServiceAccess::STOP | ServiceAccess::DELETE)?;
        let _ = svc.stop();
        svc.delete()?;
        Ok(())
    }

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> Result<()> {
        service_dispatcher::start(NAME, ffi_service_main)?;
        Ok(())
    }

    fn service_main(_args: Vec<OsString>) {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let handler = move |ev| match ev {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let Ok(h) = service_control_handler::register(NAME, handler) else { return };
        let status = |state, accept| ServiceStatus {
            service_type: ServiceType::OWN_PROCESS, current_state: state, controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0), checkpoint: 0, wait_hint: Duration::from_secs(5), process_id: None,
        };
        let _ = h.set_service_status(status(ServiceState::Running, ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN));
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        rt.spawn(async {
            if let Err(e) = super::serve().await {
                tracing::error!("Helper dừng: {e:#}");
            }
        });
        let _ = rx.recv();
        let _ = h.set_service_status(status(ServiceState::Stopped, ServiceControlAccept::empty()));
    }
}
