//! Trung tâm của Console: tìm máy học sinh, giữ kết nối, nhận ảnh, gửi lệnh, gửi file, trình chiếu.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use lc_core::proto::{self, AgentInfo, Ctrl, K_CTRL, K_FILE, K_STREAM, K_THUMB, K_TILES};
use lc_core::{auth, capture, img, tls, AGENT_PORT, BEACON_PORT};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub teacher_name: String,
    /// Chỉ hiện máy thuộc phòng này (để trống = mọi máy tìm thấy)
    pub room: String,
    /// Chu kỳ cập nhật ảnh thu nhỏ (giây)
    pub thumb_seconds: f32,
    pub thumb_width: u32,
    /// Máy thêm bằng IP (khi mạng chặn tín hiệu tìm máy)
    pub manual: Vec<String>,
    pub show_fps: u32,
    pub show_quality: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { teacher_name: "Giáo viên".into(), room: String::new(), thumb_seconds: 2.0, thumb_width: 400,
                   manual: vec![], show_fps: 12, show_quality: 70 }
    }
}

type Tx = mpsc::Sender<(u8, Arc<Vec<u8>>)>;

pub struct Student {
    pub id: String,
    pub addr: SocketAddr,
    pub info: RwLock<AgentInfo>,
    pub state: RwLock<String>, // connecting | online | offline | denied
    pub error: RwLock<String>,
    pub tx: Mutex<Option<Tx>>,
    pub streaming: AtomicBool,
    pub removed: AtomicBool,
}

#[derive(Serialize, Clone)]
pub struct StudentDto {
    pub id: String,
    pub addr: String,
    pub info: AgentInfo,
    pub state: String,
    pub error: String,
}

impl Student {
    fn dto(&self) -> StudentDto {
        StudentDto { id: self.id.clone(), addr: self.addr.to_string(), info: self.info.read().unwrap().clone(),
                     state: self.state.read().unwrap().clone(), error: self.error.read().unwrap().clone() }
    }
    fn tx(&self) -> Option<Tx> {
        self.tx.lock().unwrap().clone()
    }
}

struct Show {
    stop: Arc<AtomicBool>,
    targets: Arc<RwLock<Vec<String>>>,
    fullscreen: bool,
}

pub struct Hub {
    app: AppHandle,
    dir: PathBuf,
    key: String, // khóa riêng của giáo viên (base64)
    settings: RwLock<Settings>,
    students: Mutex<HashMap<String, Arc<Student>>>,
    show: Mutex<Option<Show>>,
    file_seq: AtomicU64,
}

impl Hub {
    pub fn new(app: AppHandle, dir: PathBuf) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&dir)?;
        let kp = dir.join("teacher.key");
        let key = match std::fs::read_to_string(&kp) {
            Ok(s) => {
                auth::signing_key_from_b64(&s)?;
                s.trim().to_string()
            }
            Err(_) => {
                let k = auth::signing_key_to_b64(&auth::new_signing_key());
                std::fs::write(&kp, &k).context("Không lưu được khóa giáo viên")?;
                k
            }
        };
        let settings: Settings = std::fs::read_to_string(dir.join("settings.json")).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Ok(Arc::new(Hub { app, dir, key, settings: RwLock::new(settings), students: Mutex::new(HashMap::new()),
                          show: Mutex::new(None), file_seq: AtomicU64::new(1) }))
    }

    fn signing(&self) -> auth::SigningKeyAlias {
        auth::signing_key_from_b64(&self.key).expect("khóa giáo viên hỏng")
    }

    pub fn teacher_code(&self) -> String {
        auth::teacher_code(&self.signing())
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn set_settings(self: &Arc<Self>, s: Settings) -> Result<()> {
        std::fs::write(self.dir.join("settings.json"), serde_json::to_string_pretty(&s)?)?;
        let manual = s.manual.clone();
        *self.settings.write().unwrap() = s;
        for m in manual {
            let _ = self.add_manual(&m);
        }
        Ok(())
    }

    pub fn students(&self) -> Vec<StudentDto> {
        let mut v: Vec<StudentDto> = self.students.lock().unwrap().values().map(|s| s.dto()).collect();
        v.sort_by(|a, b| natural(&a.info.name).cmp(&natural(&b.info.name)));
        v
    }

    pub fn showing(&self) -> bool {
        self.show.lock().unwrap().is_some()
    }

    fn emit_student(&self, s: &Student) {
        let _ = self.app.emit("student", s.dto());
    }

    pub fn start(self: Arc<Self>) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.discover().await });
        for m in self.settings().manual {
            let _ = self.add_manual(&m);
        }
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.poll().await });
    }

    // -------------------------------------------------------------- tìm máy
    async fn discover(self: Arc<Self>) {
        let sock = match tokio::net::UdpSocket::bind(("0.0.0.0", BEACON_PORT)).await {
            Ok(s) => s,
            Err(e) => {
                let _ = self.app.emit("notice", format!("Không mở được cổng tìm máy {BEACON_PORT}: {e}"));
                return;
            }
        };
        let mut buf = [0u8; 2048];
        loop {
            let Ok((n, from)) = sock.recv_from(&mut buf).await else { continue };
            let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf[..n]) else { continue };
            if v["lc"].as_u64() != Some(lc_core::PROTO_VERSION as u64) {
                continue;
            }
            let room = v["room"].as_str().unwrap_or("").to_string();
            let want = self.settings().room;
            if !want.trim().is_empty() && !room.eq_ignore_ascii_case(want.trim()) {
                continue;
            }
            let port = v["port"].as_u64().unwrap_or(AGENT_PORT as u64) as u16;
            let addr = SocketAddr::new(from.ip(), port);
            let name = v["name"].as_str().unwrap_or("").to_string();
            self.ensure(addr, name, room);
        }
    }

    pub fn add_manual(self: &Arc<Self>, s: &str) -> Result<()> {
        let s = s.trim();
        let addr: SocketAddr = s.parse().or_else(|_| format!("{s}:{AGENT_PORT}").parse())
            .map_err(|_| anyhow!("Địa chỉ không hợp lệ: {s}"))?;
        self.ensure(addr, s.to_string(), String::new());
        Ok(())
    }

    fn ensure(self: &Arc<Self>, addr: SocketAddr, name: String, room: String) {
        let id = addr.to_string();
        let mut map = self.students.lock().unwrap();
        if map.contains_key(&id) {
            return;
        }
        let st = Arc::new(Student {
            id: id.clone(), addr,
            info: RwLock::new(AgentInfo { name, room, ..Default::default() }),
            state: RwLock::new("connecting".into()), error: RwLock::new(String::new()), tx: Mutex::new(None),
            streaming: AtomicBool::new(false), removed: AtomicBool::new(false),
        });
        map.insert(id, st.clone());
        drop(map);
        self.emit_student(&st);
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.keep_connected(st).await });
    }

    pub fn remove(&self, id: &str) {
        if let Some(s) = self.students.lock().unwrap().remove(id) {
            s.removed.store(true, Ordering::Relaxed);
            *s.tx.lock().unwrap() = None;
            let _ = self.app.emit("student_removed", id.to_string());
        }
    }

    // -------------------------------------------------------------- kết nối một máy
    async fn keep_connected(self: Arc<Self>, st: Arc<Student>) {
        let mut wait = 2u64;
        while !st.removed.load(Ordering::Relaxed) {
            match self.session(&st).await {
                Ok(()) => wait = 2,
                Err(e) => {
                    *st.error.write().unwrap() = format!("{e:#}");
                }
            }
            *st.tx.lock().unwrap() = None;
            st.streaming.store(false, Ordering::Relaxed);
            {
                let mut s = st.state.write().unwrap();
                if *s != "denied" {
                    *s = "offline".into();
                }
            }
            self.emit_student(&st);
            let denied = *st.state.read().unwrap() == "denied";
            tokio::time::sleep(Duration::from_secs(if denied { 30 } else { wait })).await;
            wait = (wait * 2).min(15);
        }
    }

    async fn session(self: &Arc<Self>, st: &Arc<Student>) -> Result<()> {
        let tcp = tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(st.addr)).await
            .map_err(|_| anyhow!("không phản hồi"))??;
        tcp.set_nodelay(true)?;
        let (cfg, seen) = tls::client_config();
        let conn = tokio_rustls::TlsConnector::from(cfg);
        let name = tokio_rustls::rustls::pki_types::ServerName::try_from("labcontrol-agent")?.to_owned();
        let tls = tokio::time::timeout(Duration::from_secs(8), conn.connect(name, tcp)).await??;
        let fp = seen.lock().unwrap().clone().ok_or_else(|| anyhow!("không đọc được chứng chỉ"))?;
        let (mut rd, mut wr) = tokio::io::split(tls);

        let (k, d) = tokio::time::timeout(Duration::from_secs(8), proto::read_frame(&mut rd)).await??;
        let Ok(Ctrl::Hello { nonce, .. }) = proto::parse_ctrl(&d) else { bail!("máy học sinh trả lời lạ ({k})") };
        let auth_msg = Ctrl::Auth { teacher: self.teacher_code(), teacher_name: self.settings().teacher_name,
                                    sig: auth::sign(&self.signing(), &nonce, &fp) };
        proto::write_ctrl(&mut wr, &auth_msg).await?;
        let (_, d) = tokio::time::timeout(Duration::from_secs(8), proto::read_frame(&mut rd)).await??;
        match proto::parse_ctrl(&d)? {
            Ctrl::AuthOk { info } => *st.info.write().unwrap() = info,
            Ctrl::AuthFail { reason } => {
                *st.state.write().unwrap() = "denied".into();
                bail!("{reason}");
            }
            _ => bail!("máy học sinh trả lời lạ"),
        }
        *st.state.write().unwrap() = "online".into();
        st.error.write().unwrap().clear();
        let (tx, mut rx) = mpsc::channel::<(u8, Arc<Vec<u8>>)>(48);
        *st.tx.lock().unwrap() = Some(tx.clone());
        self.emit_student(st);

        // nếu đang trình chiếu cho cả lớp thì máy vừa kết nối cũng xem luôn
        let show = self.show.lock().unwrap().as_ref().map(|s| (s.fullscreen, s.targets.read().unwrap().contains(&st.id)));
        if let Some((fs, true)) = show {
            let _ = tx.try_send((K_CTRL, Arc::new(proto::ctrl_bytes(&Ctrl::ShowStart { fullscreen: fs }))));
        }

        let writer = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some((k, d)) = rx.recv().await {
                if proto::write_frame(&mut wr, k, &d).await.is_err() {
                    break;
                }
                if rx.is_empty() && wr.flush().await.is_err() {
                    break;
                }
            }
        });

        let res: Result<()> = async {
            loop {
                let (k, d) = proto::read_frame(&mut rd).await?;
                match k {
                    K_THUMB => {
                        let _ = self.app.emit("thumb", serde_json::json!({"id": st.id, "jpeg": B64.encode(&d)}));
                    }
                    K_STREAM => {
                        let _ = self.app.emit("frame", serde_json::json!({"id": st.id, "jpeg": B64.encode(&d)}));
                    }
                    K_CTRL => match proto::parse_ctrl(&d)? {
                        Ctrl::Status { status } => {
                            let _ = self.app.emit("status", serde_json::json!({"id": st.id, "status": status}));
                        }
                        Ctrl::FileDone { id, ok, path, error } => {
                            let _ = self.app.emit("file_done", serde_json::json!({"student": st.id, "id": id, "ok": ok,
                                                                                    "path": path, "error": error}));
                        }
                        Ctrl::Notice { text } => {
                            let _ = self.app.emit("notice", text);
                        }
                        _ => {}
                    },
                    _ => {}
                }
                if st.removed.load(Ordering::Relaxed) {
                    return Ok(());
                }
            }
        }
        .await;
        writer.abort();
        res
    }

    // -------------------------------------------------------------- cập nhật ảnh thu nhỏ định kỳ
    async fn poll(self: Arc<Self>) {
        loop {
            let s = self.settings();
            let list: Vec<Arc<Student>> = self.students.lock().unwrap().values().cloned().collect();
            let thumb = proto::ctrl_bytes(&Ctrl::GetThumb { width: s.thumb_width.clamp(160, 800), quality: 55 });
            let status = Arc::new(proto::ctrl_bytes(&Ctrl::GetStatus));
            let thumb = Arc::new(thumb);
            for st in list {
                if let Some(tx) = st.tx() {
                    if !st.streaming.load(Ordering::Relaxed) {
                        let _ = tx.try_send((K_CTRL, thumb.clone()));
                    }
                    let _ = tx.try_send((K_CTRL, status.clone()));
                }
            }
            tokio::time::sleep(Duration::from_secs_f32(s.thumb_seconds.clamp(0.5, 30.0))).await;
        }
    }

    // -------------------------------------------------------------- lệnh
    pub fn send_ctrl(&self, ids: &[String], c: &Ctrl) -> usize {
        let data = Arc::new(proto::ctrl_bytes(c));
        let map = self.students.lock().unwrap();
        let mut n = 0;
        for id in ids {
            if let Some(st) = map.get(id) {
                match c {
                    Ctrl::StreamStart { .. } => st.streaming.store(true, Ordering::Relaxed),
                    Ctrl::StreamStop => st.streaming.store(false, Ordering::Relaxed),
                    _ => {}
                }
                if let Some(tx) = st.tx() {
                    if tx.try_send((K_CTRL, data.clone())).is_ok() {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    // -------------------------------------------------------------- gửi file
    pub fn send_files(self: &Arc<Self>, ids: Vec<String>, paths: Vec<String>) -> usize {
        let targets: Vec<Arc<Student>> = {
            let map = self.students.lock().unwrap();
            ids.iter().filter_map(|i| map.get(i).cloned()).collect()
        };
        let n = targets.len() * paths.len();
        for st in targets {
            for p in &paths {
                let me = self.clone();
                let st = st.clone();
                let p = PathBuf::from(p);
                let fid = self.file_seq.fetch_add(1, Ordering::Relaxed);
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = me.send_one(&st, &p, fid).await {
                        let _ = me.app.emit("file_done", serde_json::json!({"student": st.id, "id": fid, "ok": false,
                                                                          "path": p.display().to_string(),
                                                                          "error": format!("{e:#}")}));
                    }
                });
            }
        }
        n
    }

    async fn send_one(&self, st: &Student, p: &std::path::Path, fid: u64) -> Result<()> {
        let tx = st.tx().ok_or_else(|| anyhow!("máy chưa kết nối"))?;
        let mut f = tokio::fs::File::open(p).await?;
        let size = f.metadata().await?.len();
        let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
        tx.send((K_CTRL, Arc::new(proto::ctrl_bytes(&Ctrl::FileBegin { id: fid, name, size })))).await?;
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = f.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            let mut chunk = Vec::with_capacity(n + 8);
            chunk.extend_from_slice(&fid.to_be_bytes());
            chunk.extend_from_slice(&buf[..n]);
            tx.send((K_FILE, Arc::new(chunk))).await?;
        }
        tx.send((K_CTRL, Arc::new(proto::ctrl_bytes(&Ctrl::FileEnd { id: fid })))).await?;
        Ok(())
    }

    // -------------------------------------------------------------- trình chiếu màn hình giáo viên
    pub fn start_show(self: &Arc<Self>, ids: Vec<String>, fullscreen: bool) -> Result<()> {
        self.stop_show();
        let stop = Arc::new(AtomicBool::new(false));
        let targets = Arc::new(RwLock::new(ids.clone()));
        *self.show.lock().unwrap() = Some(Show { stop: stop.clone(), targets: targets.clone(), fullscreen });
        self.send_ctrl(&ids, &Ctrl::ShowStart { fullscreen });
        let me = self.clone();
        let s = self.settings();
        std::thread::spawn(move || {
            if let Err(e) = me.show_loop(stop, targets, s.show_fps.clamp(2, 30), s.show_quality.clamp(30, 90)) {
                let _ = me.app.emit("notice", format!("Trình chiếu dừng: {e:#}"));
            }
        });
        let _ = self.app.emit("show", true);
        Ok(())
    }

    pub fn stop_show(&self) {
        if let Some(s) = self.show.lock().unwrap().take() {
            s.stop.store(true, Ordering::Relaxed);
            let ids = s.targets.read().unwrap().clone();
            self.send_ctrl(&ids, &Ctrl::ShowStop);
            let _ = self.app.emit("show", false);
        }
    }

    fn show_loop(&self, stop: Arc<AtomicBool>, targets: Arc<RwLock<Vec<String>>>, fps: u32, quality: u8) -> Result<()> {
        let mut cap = capture::Capturer::new()?;
        let mut enc = img::TileEncoder::new();
        let period = Duration::from_millis(1000 / fps as u64);
        let mut last_key = Instant::now() - Duration::from_secs(60);
        while !stop.load(Ordering::Relaxed) {
            let t0 = Instant::now();
            let f = cap.capture()?;
            let f = if f.width > 1920 { img::downscale(&f, 1920) } else { f };
            // khung đầy đủ mỗi 2 giây để máy vào sau / máy bị rớt gói vẫn có hình đúng
            let key = last_key.elapsed() > Duration::from_secs(2);
            if key {
                last_key = Instant::now();
            }
            if let Some(pkt) = enc.encode(&f, key, quality)? {
                let pkt = Arc::new(pkt);
                let ids = targets.read().unwrap().clone();
                let map = self.students.lock().unwrap();
                for id in ids {
                    if let Some(tx) = map.get(&id).and_then(|s| s.tx()) {
                        let _ = tx.try_send((K_TILES, pkt.clone())); // máy chậm thì bỏ khung, chờ khung đầy đủ kế tiếp
                    }
                }
            }
            let el = t0.elapsed();
            if el < period {
                std::thread::sleep(period - el);
            }
        }
        Ok(())
    }
}

fn natural(s: &str) -> Vec<(u8, u64, String)> {
    let mut out = Vec::new();
    let mut num = String::new();
    let mut txt = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !txt.is_empty() {
                out.push((1, 0, std::mem::take(&mut txt).to_lowercase()));
            }
            num.push(c);
        } else {
            if !num.is_empty() {
                out.push((0, num.parse().unwrap_or(0), String::new()));
                num.clear();
            }
            txt.push(c);
        }
    }
    if !num.is_empty() {
        out.push((0, num.parse().unwrap_or(0), String::new()));
    }
    if !txt.is_empty() {
        out.push((1, 0, txt.to_lowercase()));
    }
    out
}
