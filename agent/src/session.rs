//! Máy chủ nhỏ trên máy học sinh: nhận kết nối TLS từ Console giáo viên, xác thực bằng chữ ký,
//! rồi thực hiện lệnh: gửi ảnh màn hình, luồng xem/điều khiển, khóa, tin nhắn, file, trình chiếu...

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use lc_core::proto::{self, AgentInfo, Ctrl, Status, K_CTRL, K_FILE, K_STREAM, K_THUMB, K_TILES};
use lc_core::{auth, capture, img};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::TlsAcceptor;

use crate::config::Config;
use crate::input::Input;
use crate::sys;
use crate::uiman::UiMan;

pub struct Shared {
    pub cfg: Config,
    pub id: String,
    pub acceptor: TlsAcceptor,
    pub fp: String,
    pub input: Input,
    pub ui: UiMan,
    pub port: u16,
}

impl Shared {
    pub fn info(&self) -> AgentInfo {
        let (w, h) = capture::Capturer::new().map(|c| c.size()).unwrap_or((0, 0));
        AgentInfo {
            id: self.id.clone(),
            name: crate::config::display_name(&self.cfg),
            os: sys::os_name(),
            user: sys::username(),
            version: env!("CARGO_PKG_VERSION").into(),
            room: self.cfg.room.clone(),
            screen_w: w,
            screen_h: h,
        }
    }
}

pub async fn serve(sh: Arc<Shared>) -> Result<()> {
    let l = TcpListener::bind(("0.0.0.0", sh.port)).await
        .map_err(|e| anyhow::anyhow!("Không mở được cổng {} (Agent đã chạy rồi?): {e}", sh.port))?;
    tracing::info!("Agent sẵn sàng ở cổng {}", sh.port);
    loop {
        let (s, addr) = l.accept().await?;
        let sh = sh.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(s, sh).await {
                tracing::info!("Kết nối {addr} đóng: {e:#}");
            }
        });
    }
}

struct Incoming {
    file: tokio::fs::File,
    tmp: PathBuf,
    dest: PathBuf,
    left: u64,
}

fn receive_dir() -> PathBuf {
    let base = dirs::desktop_dir().or_else(dirs::home_dir).unwrap_or_else(std::env::temp_dir);
    base.join("TuGiaoVien")
}

fn safe_name(n: &str) -> String {
    let base = n.replace('\\', "/");
    let base = base.rsplit('/').next().unwrap_or("file");
    let cleaned: String = base.chars().map(|c| if "<>:\"|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    if cleaned.is_empty() { "file".into() } else { cleaned }
}

fn unique(dir: &std::path::Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (1..).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).unwrap()
}

async fn handle(stream: TcpStream, sh: Arc<Shared>) -> Result<()> {
    stream.set_nodelay(true)?;
    let tls = tokio::time::timeout(Duration::from_secs(10), sh.acceptor.accept(stream)).await??;
    let (mut rd, mut wr) = tokio::io::split(tls);

    // ----- xác thực: giáo viên phải ký nonce + dấu vân tay TLS bằng khóa riêng của mình
    let nonce = auth::random_nonce();
    proto::write_ctrl(&mut wr, &Ctrl::Hello { nonce: nonce.clone(), proto: lc_core::PROTO_VERSION, info: sh.info() }).await?;
    let (k, d) = tokio::time::timeout(Duration::from_secs(15), proto::read_frame(&mut rd)).await??;
    let teacher_name = match (k, proto::parse_ctrl(&d)) {
        (K_CTRL, Ok(Ctrl::Auth { teacher, teacher_name, sig })) if auth::verify(&sh.cfg.teachers, &teacher, &sig, &nonce, &sh.fp) => teacher_name,
        _ => {
            let _ = proto::write_ctrl(&mut wr, &Ctrl::AuthFail { reason: "Máy này không nhận mã giáo viên của bạn".into() }).await;
            bail!("giáo viên không hợp lệ");
        }
    };
    proto::write_ctrl(&mut wr, &Ctrl::AuthOk { info: sh.info() }).await?;
    tracing::info!("Giáo viên '{teacher_name}' đã kết nối");

    // ----- luồng ghi: mọi phản hồi đi qua kênh này
    let (tx, mut rx) = mpsc::channel::<(u8, Vec<u8>)>(24);
    let writer = tokio::spawn(async move {
        while let Some((k, d)) = rx.recv().await {
            if proto::write_frame(&mut wr, k, &d).await.is_err() {
                break;
            }
            if rx.is_empty() && wr.flush().await.is_err() {
                break;
            }
        }
    });

    let mut stream_stop: Option<Arc<AtomicBool>> = None;
    let mut started_show = false;
    let mut files: HashMap<u64, Incoming> = HashMap::new();

    let res: Result<()> = async {
        loop {
            let (k, d) = proto::read_frame(&mut rd).await?;
            match k {
                K_TILES => sh.ui.feed(d),
                K_FILE if d.len() >= 8 => {
                    let id = u64::from_be_bytes(d[..8].try_into().unwrap());
                    if let Some(f) = files.get_mut(&id) {
                        let data = &d[8..];
                        if (data.len() as u64) > f.left {
                            bail!("file nhận được dài hơn khai báo");
                        }
                        f.file.write_all(data).await?;
                        f.left -= data.len() as u64;
                    }
                }
                K_CTRL => {
                    let c = proto::parse_ctrl(&d)?;
                    match c {
                        Ctrl::Ping => tx.send((K_CTRL, proto::ctrl_bytes(&Ctrl::Ping))).await?,
                        Ctrl::GetStatus => {
                            let (app, title) = tokio::task::spawn_blocking(sys::active_window).await.unwrap_or_default();
                            let st = Status { user: sys::username(), app, title, locked: sh.ui.locked(),
                                              showing: sh.ui.showing(), viewed: sh.ui.viewed() };
                            tx.send((K_CTRL, proto::ctrl_bytes(&Ctrl::Status { status: st }))).await?;
                        }
                        Ctrl::GetThumb { width, quality } => {
                            let tx2 = tx.clone();
                            tokio::task::spawn_blocking(move || {
                                if let Ok(f) = capture::capture_once() {
                                    let s = img::downscale(&f, width.clamp(80, 1920));
                                    if let Ok(j) = img::jpeg(&s, quality) {
                                        let _ = tx2.try_send((K_THUMB, j));
                                    }
                                }
                            });
                        }
                        Ctrl::StreamStart { width, fps, quality } => {
                            if stream_stop.is_none() {
                                sh.ui.viewer(1);
                            }
                            if let Some(s) = stream_stop.take() {
                                s.store(true, Ordering::Relaxed);
                            }
                            let stop = Arc::new(AtomicBool::new(false));
                            stream_stop = Some(stop.clone());
                            let tx2 = tx.clone();
                            std::thread::spawn(move || stream_loop(tx2, stop, width, fps, quality));
                        }
                        Ctrl::StreamStop => {
                            if let Some(s) = stream_stop.take() {
                                s.store(true, Ordering::Relaxed);
                                sh.ui.viewer(-1);
                            }
                        }
                        // chỉ điều khiển được khi đang xem (học sinh thấy chỉ báo màu đỏ)
                        Ctrl::Input { ev } if stream_stop.is_some() => sh.input.send(ev),
                        Ctrl::Lock { message } => sh.ui.lock(&message),
                        Ctrl::Unlock => sh.ui.unlock(),
                        Ctrl::Message { text } => sh.ui.message(&text.chars().take(2000).collect::<String>()),
                        Ctrl::OpenUrl { url } => {
                            if url.starts_with("http://") || url.starts_with("https://") {
                                let _ = open::that_detached(&url);
                            }
                        }
                        Ctrl::Power { action } => sys::power(&action),
                        Ctrl::ShowStart { fullscreen } => {
                            started_show = true;
                            sh.ui.show(fullscreen);
                        }
                        Ctrl::ShowStop => {
                            started_show = false;
                            sh.ui.stop_show();
                        }
                        Ctrl::FileBegin { id, name, size } => {
                            let dir = receive_dir();
                            tokio::fs::create_dir_all(&dir).await?;
                            let dest = unique(&dir, &safe_name(&name));
                            let tmp = dest.with_extension("dangnhan");
                            let file = tokio::fs::File::create(&tmp).await?;
                            files.insert(id, Incoming { file, tmp, dest, left: size });
                        }
                        Ctrl::FileEnd { id } => {
                            if let Some(mut f) = files.remove(&id) {
                                f.file.flush().await?;
                                drop(f.file);
                                let (ok, error) = if f.left == 0 {
                                    match tokio::fs::rename(&f.tmp, &f.dest).await {
                                        Ok(_) => (true, String::new()),
                                        Err(e) => (false, e.to_string()),
                                    }
                                } else {
                                    let _ = tokio::fs::remove_file(&f.tmp).await;
                                    (false, "file nhận thiếu dữ liệu".into())
                                };
                                let done = Ctrl::FileDone { id, ok, path: f.dest.display().to_string(), error };
                                tx.send((K_CTRL, proto::ctrl_bytes(&done))).await?;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    .await;

    // ----- dọn dẹp khi giáo viên ngắt kết nối
    if let Some(s) = stream_stop.take() {
        s.store(true, Ordering::Relaxed);
        sh.ui.viewer(-1);
    }
    if started_show {
        sh.ui.stop_show(); // không để học sinh kẹt ở màn hình trình chiếu
    }
    for (_, f) in files.drain() {
        drop(f.file);
        let _ = tokio::fs::remove_file(&f.tmp).await;
    }
    writer.abort();
    res
}

/// Luồng hình cho cửa sổ "xem lớn / điều khiển" của giáo viên.
fn stream_loop(tx: mpsc::Sender<(u8, Vec<u8>)>, stop: Arc<AtomicBool>, width: u32, fps: u32, quality: u8) {
    let Ok(mut cap) = capture::Capturer::new() else { return };
    let period = Duration::from_millis(1000 / fps.clamp(1, 30) as u64);
    let mut last: Vec<u8> = Vec::new();
    let mut last_sent = Instant::now() - Duration::from_secs(10);
    while !stop.load(Ordering::Relaxed) && !tx.is_closed() {
        let t0 = Instant::now();
        if let Ok(f) = cap.capture() {
            let s = img::downscale(&f, width.clamp(320, 2560));
            // màn hình không đổi thì chỉ gửi lại mỗi 2 giây
            if s.bgra != last || last_sent.elapsed() > Duration::from_secs(2) {
                if let Ok(j) = img::jpeg(&s, quality) {
                    if tx.try_send((K_STREAM, j)).is_ok() {
                        last = s.bgra;
                        last_sent = Instant::now();
                    }
                }
            }
        }
        let el = t0.elapsed();
        if el < period {
            std::thread::sleep(period - el);
        }
    }
}
