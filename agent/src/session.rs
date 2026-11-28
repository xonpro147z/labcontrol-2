//! Một phiên làm việc với một máy giáo viên (sau khi đã bắt tay xong ở link.rs):
//! gửi ảnh màn hình, luồng xem/điều khiển, khóa, tin nhắn, file, trình chiếu, chặn, bài tập, trắc nghiệm...

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use lc_core::proto::{self, AgentInfo, Ctrl, Status, K_CTRL, K_FILE, K_STREAM, K_THUMB, K_TILES};
use lc_core::{auth, capture, img};
use serde_json::json;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc};

use crate::config::{self, PublicConfig};
use crate::input::Input;
use crate::policy::{self, Guard};
use crate::uiman::UiMan;
use crate::{helper, sys, tasks};

pub struct Shared {
    pub id: String,
    pub input: Input,
    pub ui: UiMan,
    pub guard: Guard,
    /// Tin gửi tới mọi máy giáo viên đang nối (câu trả lời trắc nghiệm, thông báo)
    pub outbox: broadcast::Sender<Ctrl>,
}

impl Shared {
    pub fn info(&self, cfg: &PublicConfig) -> AgentInfo {
        let (w, h) = capture::Capturer::new().map(|c| c.size()).unwrap_or((0, 0));
        AgentInfo {
            id: self.id.clone(),
            name: config::display_name(&cfg.name),
            os: sys::os_name(),
            user: sys::username(),
            version: env!("CARGO_PKG_VERSION").into(),
            room: cfg.room.clone(),
            screen_w: w,
            screen_h: h,
            platform: std::env::consts::OS.into(),
            macs: sys::mac_addresses(),
        }
    }
}

struct Incoming {
    file: tokio::fs::File,
    tmp: PathBuf,
    dest: PathBuf,
    left: u64,
    /// Gói cập nhật Agent (không phải file gửi cho học sinh)
    update: Option<UpdateMeta>,
}

struct UpdateMeta {
    version: String,
    platform: String,
    sha256: String,
    ts: u64,
    sig: String,
}

fn receive_dir() -> PathBuf {
    tasks::desktop().join("TuGiaoVien")
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1 << 40);

pub async fn run<R, W>(mut rd: R, mut wr: W, sh: Arc<Shared>, teacher: String) -> Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
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
    // chuyển tin chung (trắc nghiệm, thông báo) tới máy giáo viên này
    let fwd = {
        let tx = tx.clone();
        let mut sub = sh.outbox.subscribe();
        tokio::spawn(async move {
            loop {
                match sub.recv().await {
                    Ok(c) => {
                        if tx.send((K_CTRL, proto::ctrl_bytes(&c))).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        })
    };

    let mut stream_stop: Option<Arc<AtomicBool>> = None;
    let mut started_show = false;
    let mut files: HashMap<u64, Incoming> = HashMap::new();
    let mut last_seen = Instant::now();

    let res: Result<()> = async {
        loop {
            // máy giáo viên hỏi trạng thái mỗi vài giây; im lặng quá lâu là mất kết nối
            let (k, d) = match tokio::time::timeout(Duration::from_secs(60), proto::read_frame(&mut rd)).await {
                Ok(r) => r?,
                Err(_) => bail!("máy giáo viên im lặng quá lâu ({}s)", last_seen.elapsed().as_secs()),
            };
            last_seen = Instant::now();
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
                    // lệnh lạ (máy giáo viên bản mới hơn) thì bỏ qua, không ngắt kết nối
                    let Ok(c) = proto::parse_ctrl(&d) else {
                        tracing::debug!("bỏ qua lệnh không hiểu");
                        continue;
                    };
                    handle(c, &sh, &tx, &teacher, &mut stream_stop, &mut started_show, &mut files).await?;
                }
                _ => {}
            }
        }
    }
    .await;

    // ----- dọn dẹp khi máy giáo viên ngắt kết nối
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
    fwd.abort();
    writer.abort();
    res
}

fn send_ctrl(tx: &mpsc::Sender<(u8, Vec<u8>)>, c: &Ctrl) {
    let _ = tx.try_send((K_CTRL, proto::ctrl_bytes(c)));
}

#[allow(clippy::too_many_arguments)]
async fn handle(
    c: Ctrl,
    sh: &Arc<Shared>,
    tx: &mpsc::Sender<(u8, Vec<u8>)>,
    teacher: &str,
    stream_stop: &mut Option<Arc<AtomicBool>>,
    started_show: &mut bool,
    files: &mut HashMap<u64, Incoming>,
) -> Result<()> {
    match c {
        Ctrl::Ping => tx.send((K_CTRL, proto::ctrl_bytes(&Ctrl::Pong))).await?,
        Ctrl::GetStatus => {
            let (app, title) = tokio::task::spawn_blocking(sys::active_window).await.unwrap_or_default();
            let pol = sh.guard.current();
            sh.ui.set_exam(pol.exam);
            let st = Status {
                user: sys::username(), app, title,
                locked: sh.ui.locked(), showing: sh.ui.showing(), viewed: sh.ui.viewed(),
                exam: pol.exam, web_blocked: sh.guard.web_blocked(), tools_blocked: pol.block_tools || pol.exam,
                helper: helper::has_helper().await,
                muted: pol.mute,
            };
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
            *stream_stop = Some(stop.clone());
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
            *started_show = true;
            sh.ui.show(fullscreen);
        }
        Ctrl::ShowStop => {
            *started_show = false;
            sh.ui.stop_show();
        }
        Ctrl::FileBegin { id, name, size } => {
            let (sub, file) = tasks::safe_rel(&name);
            let dir = receive_dir().join(sub);
            tokio::fs::create_dir_all(&dir).await?;
            let dest = tasks::unique(&dir, &file);
            let tmp = dest.with_extension("dangnhan");
            let file = tokio::fs::File::create(&tmp).await?;
            files.insert(id, Incoming { file, tmp, dest, left: size, update: None });
        }
        Ctrl::UpdateBegin { id, version, platform, size, sha256, ts, sig } => {
            let msg = auth::update_message(&platform, &version, &sha256);
            let err = if platform != std::env::consts::OS {
                Some(l!("gói dành cho {platform}", "package is for {platform}"))
            } else if !auth::verify_policy(&[teacher.to_string()], &msg, ts, &sig) {
                Some(l!("chữ ký không hợp lệ", "invalid signature"))
            } else if size > 200 << 20 {
                Some(l!("gói quá lớn", "package too large"))
            } else {
                None
            };
            if let Some(error) = err {
                send_ctrl(tx, &Ctrl::UpdateDone { ok: false, version, error });
                return Ok(());
            }
            let tmp = std::env::temp_dir().join(format!("labcontrol-update-{id}.bin"));
            let file = tokio::fs::File::create(&tmp).await?;
            files.insert(id, Incoming { file, tmp: tmp.clone(), dest: tmp, left: size,
                                        update: Some(UpdateMeta { version, platform, sha256, ts, sig }) });
        }
        Ctrl::UpdateEnd { id } => {
            let Some(mut f) = files.remove(&id) else { return Ok(()) };
            let Some(m) = f.update.take() else { return Ok(()) };
            f.file.flush().await?;
            drop(f.file);
            let data = tokio::fs::read(&f.tmp).await.unwrap_or_default();
            let _ = tokio::fs::remove_file(&f.tmp).await;
            let r: Result<()> = async {
                if f.left != 0 || auth::sha256_hex(&data) != m.sha256 {
                    bail!("{}", l!("file nhận thiếu hoặc hỏng", "the received file is incomplete or damaged"));
                }
                use base64::Engine;
                let req = json!({"op": "update", "platform": m.platform, "version": m.version, "sha256": m.sha256,
                                 "ts": m.ts, "sig": m.sig, "data": base64::engine::general_purpose::STANDARD.encode(&data)});
                helper::machine_wait(req, Duration::from_secs(90)).await?;
                Ok(())
            }.await;
            match r {
                Ok(()) => {
                    send_ctrl(tx, &Ctrl::UpdateDone { ok: true, version: m.version.clone(), error: String::new() });
                    tracing::info!("Đã cập nhật lên {}, khởi động lại Agent", m.version);
                    let ui = sh.ui.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(1200)).await;
                        crate::restart(&ui);
                    });
                }
                Err(e) => send_ctrl(tx, &Ctrl::UpdateDone { ok: false, version: m.version, error: format!("{e:#}") }),
            }
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
                    (false, l!("file nhận thiếu dữ liệu", "the received file is incomplete"))
                };
                if ok {
                    announce_files(sh);
                }
                tx.send((K_CTRL, proto::ctrl_bytes(&Ctrl::FileDone { id, ok, path: f.dest.display().to_string(), error }))).await?;
            }
        }
        // ----------------------------------------------------------- giới hạn
        Ctrl::SetPolicy { policy, ts, sig } => {
            let pj = serde_json::to_string(&policy)?;
            if !auth::verify_policy(&[teacher.to_string()], &pj, ts, &sig) {
                send_ctrl(tx, &Ctrl::Notice { text: l!("Chữ ký chính sách không hợp lệ", "Invalid policy signature") });
                return Ok(());
            }
            // chặn web: nhờ Helper (quyền cao) sửa file hosts
            let web_ok = match helper::machine(json!({"op": "policy", "policy": pj, "ts": ts, "sig": sig})).await {
                Ok(_) => true,
                Err(e) => {
                    if !policy.block_web.is_empty() {
                        send_ctrl(tx, &Ctrl::Notice { text: l!("Không chặn được web: {e:#}", "Cannot block websites: {e:#}") });
                    }
                    false
                }
            };
            sh.ui.set_exam(policy.exam);
            sh.guard.set(policy, web_ok);
        }
        Ctrl::CloseApp { name } => {
            let n = tokio::task::spawn_blocking(move || {
                let k = policy::close_app(&name);
                (name, k)
            }).await;
            if let Ok((name, k)) = n {
                send_ctrl(tx, &Ctrl::Notice { text: if k > 0 { l!("Đã đóng {name}", "Closed {name}") } else { l!("Không thấy {name} đang chạy", "{name} is not running") } });
            }
        }
        Ctrl::RunApp { cmd } => {
            let r = tokio::task::spawn_blocking({
                let cmd = cmd.clone();
                move || sys::run_app(&cmd)
            }).await;
            if let Ok(Err(e)) = r {
                send_ctrl(tx, &Ctrl::Notice { text: l!("Không mở được {cmd}: {e}", "Cannot open {cmd}: {e}") });
            }
        }
        // ----------------------------------------------------------- bài tập
        Ctrl::Assign { task, title, text } => {
            match tasks::assign(&task, &title, &text) {
                Ok(_) => {
                    let msg = if text.trim().is_empty() {
                        l!("Bài tập mới: {title}\n\nLưu bài làm vào thư mục BaiLam trên màn hình nền.", "New assignment: {title}\n\nSave your work in the BaiLam folder on the desktop.")
                    } else {
                        format!("{title}\n\n{text}")
                    };
                    sh.ui.message(&msg.chars().take(2000).collect::<String>());
                }
                Err(e) => send_ctrl(tx, &Ctrl::Notice { text: l!("Không tạo được thư mục bài làm: {e}", "Cannot create the work folder: {e}") }),
            }
        }
        Ctrl::Collect { task, title } => {
            let tx2 = tx.clone();
            tokio::spawn(async move {
                let (files, error) = match collect(&tx2, &task, &title).await {
                    Ok(n) => (n, String::new()),
                    Err(e) => (0, format!("{e:#}")),
                };
                let _ = tx2.send((K_CTRL, proto::ctrl_bytes(&Ctrl::CollectEnd { task, files, error }))).await;
            });
        }
        // ----------------------------------------------------------- trắc nghiệm
        Ctrl::QuizStart { mut quiz } => {
            for q in &mut quiz.questions {
                q.answer = None; // đề phòng: không bao giờ hiện đáp án
            }
            sh.ui.quiz_start(quiz)
        }
        Ctrl::QuizStop { id } => sh.ui.quiz_stop(&id),
        Ctrl::SetLang { lang } => crate::i18n::set(&lang),
        // ----------------------------------------------------------- quản trị
        Ctrl::SetPairPassword { password, ts, sig } => {
            let r = helper::machine(json!({"op": "set_password", "password": password, "ts": ts, "sig": sig})).await;
            send_ctrl(tx, &Ctrl::Notice { text: match r {
                Ok(_) => l!("Đã đổi mật khẩu phòng", "Room password changed"),
                Err(e) => l!("Không đổi được mật khẩu phòng: {e:#}", "Cannot change the room password: {e:#}"),
            } });
        }
        _ => {}
    }
    Ok(())
}

/// Báo cho học sinh khi nhận xong một đợt file (gộp các file đến cách nhau dưới 2 giây thành một thông báo)
/// và mở thư mục TuGiaoVien.
fn announce_files(sh: &Arc<Shared>) {
    static STATE: std::sync::Mutex<(u64, u32)> = std::sync::Mutex::new((0, 0));
    let gen = {
        let mut s = STATE.lock().unwrap();
        s.0 += 1;
        s.1 += 1;
        s.0
    };
    let sh = sh.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let n = {
            let mut s = STATE.lock().unwrap();
            if s.0 != gen {
                return; // còn file đang đến, đợi file cuối
            }
            std::mem::take(&mut s.1)
        };
        let dir = receive_dir();
        sh.ui.message(&l!("Giáo viên vừa gửi {n} file cho em.\n\nFile nằm trong thư mục TuGiaoVien trên màn hình nền.", "The teacher just sent you {n} files.\n\nThey are in the TuGiaoVien folder on your desktop."));
        if !sh.guard.current().exam {
            let _ = open::that_detached(&dir);
        }
    });
}

async fn collect(tx: &mpsc::Sender<(u8, Vec<u8>)>, task: &str, title: &str) -> Result<u32> {
    let dir = tasks::task_dir(task, title);
    if !dir.exists() {
        bail!("{}", l!("chưa có thư mục bài làm", "no work folder yet"));
    }
    let list = tokio::task::spawn_blocking(move || tasks::collect_list(&dir)).await??;
    let mut n = 0;
    for (rel, path, size) in list {
        let Ok(mut f) = tokio::fs::File::open(&path).await else { continue };
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        tx.send((K_CTRL, proto::ctrl_bytes(&Ctrl::CollectFile { task: task.into(), id, name: rel, size }))).await?;
        let mut left = size;
        let mut buf = vec![0u8; 256 * 1024];
        while left > 0 {
            let want = (left as usize).min(buf.len());
            let got = f.read(&mut buf[..want]).await?;
            if got == 0 {
                // file bị cắt ngắn trong lúc gửi: bù số 0 cho đủ kích thước đã báo
                buf[..want].fill(0);
                let mut c = id.to_be_bytes().to_vec();
                c.extend_from_slice(&buf[..want]);
                tx.send((K_FILE, c)).await?;
                left -= want as u64;
                continue;
            }
            let mut c = id.to_be_bytes().to_vec();
            c.extend_from_slice(&buf[..got]);
            tx.send((K_FILE, c)).await?;
            left -= got as u64;
        }
        n += 1;
    }
    Ok(n)
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
