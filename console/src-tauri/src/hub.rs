//! Trung tâm của Console (máy giáo viên).
//!
//! Từ bản 3, máy giáo viên là "trạm": lắng nghe TCP 47900 và trả lời tín hiệu tìm máy UDP 47901.
//! Máy học sinh tự tìm và tự nối về, ghép lần đầu bằng **mật khẩu phòng**. Chỉ máy giáo viên cần mở tường lửa.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use lc_core::handshake::{self, Outcome, TeacherSide};
use lc_core::proto::{self, AgentInfo, Ctrl, Discover, Policy, Quiz, Status, K_CTRL, K_FILE, K_STREAM, K_THUMB, K_TILES};
use lc_core::{auth, capture, img, tls, DISCOVER_PORT, PROTO_VERSION, TEACHER_PORT};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub teacher_name: String,
    /// Tên phòng: chỉ nhận máy cùng phòng (để trống = mọi máy)
    pub room: String,
    /// Mật khẩu phòng để ghép máy học sinh mới
    pub room_password: String,
    /// Cho phép ghép máy mới bằng mật khẩu phòng
    pub accept_new: bool,
    /// Chu kỳ cập nhật ảnh thu nhỏ (giây)
    pub thumb_seconds: f32,
    pub thumb_width: u32,
    pub show_fps: u32,
    pub show_quality: u8,
    /// Danh sách web hay chặn (giáo viên chọn nhanh)
    pub web_list: Vec<String>,
    /// Danh sách ứng dụng hay chặn
    pub app_list: Vec<String>,
    pub dark: bool,
    /// Số cột của sơ đồ phòng
    pub seat_cols: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            teacher_name: "Giáo viên".into(), room: String::new(), room_password: String::new(), accept_new: true,
            thumb_seconds: 2.0, thumb_width: 400, show_fps: 12, show_quality: 70,
            web_list: ["youtube.com", "facebook.com", "tiktok.com", "messenger.com", "zalo.me", "chatgpt.com"]
                .iter().map(|s| s.to_string()).collect(),
            app_list: ["zalo.exe", "telegram.exe", "discord.exe", "chrome.exe", "msedge.exe", "firefox.exe"]
                .iter().map(|s| s.to_string()).collect(),
            dark: false, seat_cols: 6,
        }
    }
}

/// Máy đã từng nối về (lưu lại để hiện cả máy đang tắt, và vị trí trong sơ đồ phòng)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Machine {
    pub name: String,
    pub ip: String,
    /// Vị trí trong sơ đồ phòng (cột, hàng); None = chưa xếp
    pub seat: Option<(u32, u32)>,
    pub last_seen: u64,
    /// Tên học sinh giáo viên gõ (nếu muốn khác tên đăng nhập)
    pub label: String,
}

type Tx = mpsc::Sender<(u8, Arc<Vec<u8>>)>;

pub struct Student {
    pub id: String,
    pub addr: RwLock<String>,
    pub info: RwLock<AgentInfo>,
    pub online: AtomicBool,
    pub status: RwLock<Option<Status>>,
    pub tx: Mutex<Option<(u64, Tx)>>,
    pub streaming: AtomicBool,
}

#[derive(Serialize, Clone)]
pub struct StudentDto {
    pub id: String,
    pub addr: String,
    pub info: AgentInfo,
    pub state: String,
    pub status: Option<Status>,
    pub seat: Option<(u32, u32)>,
    pub label: String,
}

impl Student {
    fn tx(&self) -> Option<Tx> {
        self.tx.lock().unwrap().as_ref().map(|(_, t)| t.clone())
    }
}

#[derive(Serialize, Clone)]
pub struct Pending {
    pub id: String,
    pub name: String,
    pub user: String,
    pub ip: String,
    pub reason: String,
    pub at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub text: String,
    pub created: u64,
    /// số file đã thu theo máy
    #[serde(default)]
    pub collected: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QuizResult {
    pub answers: Vec<i32>,
    pub done: bool,
    pub correct: u32,
    pub user: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuizRun {
    pub quiz: Quiz,
    pub started: u64,
    pub running: bool,
    pub results: HashMap<String, QuizResult>,
}

struct Show {
    stop: Arc<AtomicBool>,
    targets: Arc<RwLock<Vec<String>>>,
    fullscreen: bool,
}

struct Incoming {
    file: tokio::fs::File,
    path: PathBuf,
    left: u64,
}

pub struct Hub {
    app: AppHandle,
    dir: PathBuf,
    out_dir: PathBuf,
    key: auth::SigningKeyAlias,
    fp: String,
    acceptor: tokio_rustls::TlsAcceptor,
    settings: RwLock<Settings>,
    students: Mutex<HashMap<String, Arc<Student>>>,
    machines: Mutex<HashMap<String, Machine>>,
    pending: Mutex<HashMap<String, Pending>>,
    show: Mutex<Option<Show>>,
    policy: RwLock<Policy>,
    tasks: Mutex<Vec<Task>>,
    quiz: Mutex<Option<QuizRun>>,
    file_seq: AtomicU64,
    conn_seq: AtomicU64,
    listen_error: RwLock<String>,
}

fn read_json<T: for<'de> Deserialize<'de> + Default>(p: &Path) -> T {
    std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn write_json<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(v)?)?;
    std::fs::rename(&tmp, p)?;
    Ok(())
}

fn safe_part(s: &str) -> String {
    let c: String = s.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
    let c = c.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    if c.is_empty() { "_".into() } else { c.chars().take(100).collect() }
}

/// Đường dẫn tương đối an toàn (không thoát ra ngoài thư mục)
fn safe_rel(rel: &str) -> PathBuf {
    let mut p = PathBuf::new();
    for c in Path::new(&rel.replace('\\', "/")).components() {
        if let Component::Normal(s) = c {
            p.push(safe_part(&s.to_string_lossy()));
        }
    }
    if p.as_os_str().is_empty() {
        p.push("file");
    }
    p
}

impl Hub {
    pub fn new(app: AppHandle, dir: PathBuf, out_dir: PathBuf) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&dir)?;
        let kp = dir.join("teacher.key");
        let key = match std::fs::read_to_string(&kp) {
            Ok(s) => auth::signing_key_from_b64(s.trim())?,
            Err(_) => {
                let k = auth::new_signing_key();
                std::fs::write(&kp, auth::signing_key_to_b64(&k)).context("Không lưu được khóa giáo viên")?;
                k
            }
        };
        let (srv, fp) = tls::server_config(&dir)?;
        let mut settings: Settings = read_json(&dir.join("settings.json"));
        if settings.room_password.is_empty() {
            // mật khẩu phòng mặc định: 6 chữ số ngẫu nhiên, giáo viên đổi được trong Cài đặt
            let n = u32::from_le_bytes(auth::random_nonce().as_bytes()[..4].try_into().unwrap()) % 900_000 + 100_000;
            settings.room_password = n.to_string();
            let _ = write_json(&dir.join("settings.json"), &settings);
        }
        let machines: HashMap<String, Machine> = read_json(&dir.join("machines.json"));
        let tasks: Vec<Task> = read_json(&dir.join("tasks.json"));
        let students = machines.iter().map(|(id, m)| {
            (id.clone(), Arc::new(Student {
                id: id.clone(), addr: RwLock::new(m.ip.clone()),
                info: RwLock::new(AgentInfo { id: id.clone(), name: m.name.clone(), ..Default::default() }),
                online: AtomicBool::new(false), status: RwLock::new(None), tx: Mutex::new(None),
                streaming: AtomicBool::new(false),
            }))
        }).collect();
        Ok(Arc::new(Hub {
            app, dir, out_dir, key, fp, acceptor: tokio_rustls::TlsAcceptor::from(srv),
            settings: RwLock::new(settings), students: Mutex::new(students), machines: Mutex::new(machines),
            pending: Mutex::new(HashMap::new()), show: Mutex::new(None), policy: RwLock::new(Policy::default()),
            tasks: Mutex::new(tasks), quiz: Mutex::new(None), file_seq: AtomicU64::new(1), conn_seq: AtomicU64::new(1),
            listen_error: RwLock::new(String::new()),
        }))
    }

    pub fn teacher_code(&self) -> String {
        auth::teacher_code(&self.key)
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn set_settings(&self, s: Settings) -> Result<()> {
        if s.room_password.trim().chars().count() < 4 {
            bail!("Mật khẩu phòng phải có ít nhất 4 ký tự");
        }
        write_json(&self.dir.join("settings.json"), &s)?;
        *self.settings.write().unwrap() = s;
        Ok(())
    }

    pub fn listen_error(&self) -> String {
        self.listen_error.read().unwrap().clone()
    }

    pub fn out_dir(&self) -> PathBuf {
        self.out_dir.clone()
    }

    fn dto(&self, s: &Student) -> StudentDto {
        let m = self.machines.lock().unwrap().get(&s.id).cloned().unwrap_or_default();
        StudentDto {
            id: s.id.clone(), addr: s.addr.read().unwrap().clone(), info: s.info.read().unwrap().clone(),
            state: if s.online.load(Ordering::Relaxed) { "online".into() } else { "offline".into() },
            status: s.status.read().unwrap().clone(), seat: m.seat, label: m.label,
        }
    }

    pub fn students(&self) -> Vec<StudentDto> {
        let list: Vec<Arc<Student>> = self.students.lock().unwrap().values().cloned().collect();
        let mut v: Vec<StudentDto> = list.iter().map(|s| self.dto(s)).collect();
        v.sort_by(|a, b| natural(&a.info.name).cmp(&natural(&b.info.name)));
        v
    }

    pub fn pending(&self) -> Vec<Pending> {
        let mut v: Vec<Pending> = self.pending.lock().unwrap().values().cloned().collect();
        v.sort_by_key(|p| std::cmp::Reverse(p.at));
        v
    }

    pub fn showing(&self) -> bool {
        self.show.lock().unwrap().is_some()
    }

    pub fn policy(&self) -> Policy {
        self.policy.read().unwrap().clone()
    }

    pub fn tasks(&self) -> Vec<Task> {
        self.tasks.lock().unwrap().clone()
    }

    pub fn quiz(&self) -> Option<QuizRun> {
        self.quiz.lock().unwrap().clone()
    }

    fn emit_student(&self, s: &Student) {
        let _ = self.app.emit("student", self.dto(s));
    }

    fn notice(&self, text: impl Into<String>) {
        let _ = self.app.emit("notice", text.into());
    }

    fn save_machines(&self) {
        let m = self.machines.lock().unwrap().clone();
        let _ = write_json(&self.dir.join("machines.json"), &m);
    }

    pub fn start(self: Arc<Self>) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.answer_seek().await });
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.listen().await });
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.poll().await });
        let me = self.clone();
        tauri::async_runtime::spawn(async move { me.refresh_policy().await });
    }

    // -------------------------------------------------------------- trả lời máy học sinh đang tìm
    async fn answer_seek(self: Arc<Self>) {
        let sock = match tokio::net::UdpSocket::bind(("0.0.0.0", DISCOVER_PORT)).await {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("Không mở được cổng UDP {DISCOVER_PORT} ({e}). Có thể đang mở hai Console?");
                *self.listen_error.write().unwrap() = msg.clone();
                self.notice(msg);
                return;
            }
        };
        let mut buf = [0u8; 2048];
        loop {
            let Ok((n, from)) = sock.recv_from(&mut buf).await else { continue };
            let Ok(d) = serde_json::from_slice::<Discover>(&buf[..n]) else { continue };
            if d.kind != "seek" || d.lc != PROTO_VERSION {
                continue;
            }
            let s = self.settings();
            if !s.room.trim().is_empty() && !d.room.trim().is_empty() && !s.room.trim().eq_ignore_ascii_case(d.room.trim()) {
                continue;
            }
            let r = Discover { lc: PROTO_VERSION, kind: "here".into(), room: s.room.trim().into(), port: TEACHER_PORT,
                               teacher: self.teacher_code(), name: s.teacher_name.clone() };
            let _ = sock.send_to(&serde_json::to_vec(&r).unwrap_or_default(), from).await;
        }
    }

    // -------------------------------------------------------------- nhận kết nối
    async fn listen(self: Arc<Self>) {
        let l = match tokio::net::TcpListener::bind(("0.0.0.0", TEACHER_PORT)).await {
            Ok(l) => l,
            Err(e) => {
                let msg = format!("Không mở được cổng {TEACHER_PORT} ({e}). Có thể đang mở hai Console?");
                *self.listen_error.write().unwrap() = msg.clone();
                self.notice(msg);
                return;
            }
        };
        loop {
            let Ok((tcp, addr)) = l.accept().await else { continue };
            let me = self.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = me.session(tcp, addr).await {
                    eprintln!("phiên {addr}: {e:#}");
                }
            });
        }
    }

    async fn session(self: &Arc<Self>, tcp: tokio::net::TcpStream, addr: SocketAddr) -> Result<()> {
        tcp.set_nodelay(true)?;
        let tls = tokio::time::timeout(Duration::from_secs(10), self.acceptor.accept(tcp)).await??;
        let (mut rd, mut wr) = tokio::io::split(tls);
        let s = self.settings();
        let side = TeacherSide {
            key: &self.key, name: &s.teacher_name, room: s.room.trim(), fp: &self.fp,
            password: if s.accept_new { Some(s.room_password.as_str()) } else { None },
        };
        let (info, outcome) = match handshake::teacher(&mut rd, &mut wr, &side).await {
            Ok(x) => x,
            Err(e) => {
                if let Some(i) = e.info {
                    let reason = if e.reason.contains("chưa ghép") && !s.accept_new {
                        "máy chưa ghép (đang tắt nhận máy mới)".to_string()
                    } else {
                        e.reason.clone()
                    };
                    let p = Pending { id: i.id.clone(), name: i.name, user: i.user, ip: addr.ip().to_string(), reason, at: now_ms() };
                    self.pending.lock().unwrap().insert(i.id, p.clone());
                    let _ = self.app.emit("pending", self.pending());
                }
                bail!("{}", e.reason);
            }
        };
        if self.pending.lock().unwrap().remove(&info.id).is_some() {
            let _ = self.app.emit("pending", self.pending());
        }
        if outcome == Outcome::Paired {
            self.notice(format!("Đã ghép máy {} bằng mật khẩu phòng", info.name));
        }
        let id = info.id.clone();
        {
            let mut m = self.machines.lock().unwrap();
            let e = m.entry(id.clone()).or_default();
            e.name = info.name.clone();
            e.ip = addr.ip().to_string();
            e.last_seen = now_ms();
        }
        self.save_machines();
        let st = {
            let mut map = self.students.lock().unwrap();
            map.entry(id.clone()).or_insert_with(|| Arc::new(Student {
                id: id.clone(), addr: RwLock::new(String::new()), info: RwLock::new(AgentInfo::default()),
                online: AtomicBool::new(false), status: RwLock::new(None), tx: Mutex::new(None),
                streaming: AtomicBool::new(false),
            })).clone()
        };
        *st.info.write().unwrap() = info;
        *st.addr.write().unwrap() = addr.ip().to_string();
        let (tx, mut rx) = mpsc::channel::<(u8, Arc<Vec<u8>>)>(48);
        let conn = self.conn_seq.fetch_add(1, Ordering::Relaxed);
        *st.tx.lock().unwrap() = Some((conn, tx.clone())); // phiên cũ (nếu còn) mất kênh gửi và tự đóng
        st.online.store(true, Ordering::Relaxed);
        st.streaming.store(false, Ordering::Relaxed);
        self.emit_student(&st);

        // máy vừa vào lớp: áp dụng trạng thái hiện tại của lớp
        let show = self.show.lock().unwrap().as_ref().map(|s| (s.fullscreen, s.targets.read().unwrap().contains(&st.id)));
        if let Some((fs, true)) = show {
            let _ = tx.try_send((K_CTRL, Arc::new(proto::ctrl_bytes(&Ctrl::ShowStart { fullscreen: fs }))));
        }
        // luôn gửi chính sách hiện tại (kể cả "không giới hạn"), để máy còn giữ giới hạn cũ
        // từ lần Console trước được bỏ ngay
        let pol = self.policy();
        let _ = tx.try_send((K_CTRL, Arc::new(proto::ctrl_bytes(&self.signed_policy(&pol)))));
        if let Some(q) = self.quiz.lock().unwrap().as_ref().filter(|q| q.running && q.results.contains_key(&st.id)) {
            if !q.results[&st.id].done {
                let _ = tx.try_send((K_CTRL, Arc::new(proto::ctrl_bytes(&Ctrl::QuizStart { quiz: strip(&q.quiz) }))));
            }
        }

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

        let mut incoming: HashMap<u64, (String, Incoming)> = HashMap::new();
        let res: Result<()> = async {
            loop {
                let (k, d) = match tokio::time::timeout(Duration::from_secs(45), proto::read_frame(&mut rd)).await {
                    Ok(r) => r?,
                    Err(_) => bail!("máy học sinh không trả lời"),
                };
                if st.tx.lock().unwrap().as_ref().map(|(c, _)| *c) != Some(conn) {
                    return Ok(()); // đã có phiên mới thay thế
                }
                match k {
                    K_THUMB => {
                        let _ = self.app.emit("thumb", json!({"id": st.id, "jpeg": B64.encode(&d)}));
                    }
                    K_STREAM => {
                        let _ = self.app.emit("frame", json!({"id": st.id, "jpeg": B64.encode(&d)}));
                    }
                    K_FILE if d.len() >= 8 => {
                        let fid = u64::from_be_bytes(d[..8].try_into().unwrap());
                        let mut finished = None;
                        if let Some((task, f)) = incoming.get_mut(&fid) {
                            let data = &d[8..];
                            let n = (data.len() as u64).min(f.left) as usize;
                            f.file.write_all(&data[..n]).await?;
                            f.left -= n as u64;
                            if f.left == 0 {
                                finished = Some(task.clone());
                            }
                        }
                        if let Some(task) = finished {
                            if let Some((_, mut f)) = incoming.remove(&fid) {
                                f.file.flush().await?;
                                self.collected(&task, &st, &f.path);
                            }
                        }
                    }
                    K_CTRL => match proto::parse_ctrl(&d)? {
                        Ctrl::Status { status } => {
                            *st.status.write().unwrap() = Some(status.clone());
                            let _ = self.app.emit("status", json!({"id": st.id, "status": status}));
                        }
                        Ctrl::FileDone { id, ok, path, error } => {
                            let _ = self.app.emit("file_done", json!({"student": st.id, "id": id, "ok": ok, "path": path, "error": error}));
                        }
                        Ctrl::Notice { text } => {
                            let name = st.info.read().unwrap().name.clone();
                            self.notice(format!("{name}: {text}"));
                        }
                        Ctrl::CollectFile { task, id: fid, name, size } => {
                            let path = self.collect_path(&task, &st, &name);
                            if let Some(p) = path.parent() {
                                tokio::fs::create_dir_all(p).await?;
                            }
                            let file = tokio::fs::File::create(&path).await?;
                            if size == 0 {
                                drop(file);
                                self.collected(&task, &st, &path);
                            } else {
                                incoming.insert(fid, (task, Incoming { file, path, left: size }));
                            }
                        }
                        Ctrl::CollectEnd { task, files, error } => {
                            let name = st.info.read().unwrap().name.clone();
                            let _ = self.app.emit("collect", json!({"task": task, "student": st.id, "name": name,
                                                                     "done": true, "files": files, "error": error}));
                        }
                        Ctrl::QuizAnswer { id, answers, user, done } => self.quiz_answer(&st, &id, answers, user, done),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        .await;
        writer.abort();
        // chỉ đánh dấu ngoại tuyến nếu đây vẫn là phiên mới nhất của máy
        let mine = {
            let mut t = st.tx.lock().unwrap();
            if t.as_ref().map(|(c, _)| *c) == Some(conn) {
                *t = None;
                true
            } else {
                false
            }
        };
        if mine {
            st.online.store(false, Ordering::Relaxed);
            st.streaming.store(false, Ordering::Relaxed);
            self.emit_student(&st);
        }
        res
    }

    // -------------------------------------------------------------- cập nhật ảnh thu nhỏ định kỳ
    async fn poll(self: Arc<Self>) {
        loop {
            let s = self.settings();
            let list: Vec<Arc<Student>> = self.students.lock().unwrap().values().cloned().collect();
            let thumb = Arc::new(proto::ctrl_bytes(&Ctrl::GetThumb { width: s.thumb_width.clamp(160, 800), quality: 55 }));
            let status = Arc::new(proto::ctrl_bytes(&Ctrl::GetStatus));
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
    pub fn online_ids(&self) -> Vec<String> {
        self.students.lock().unwrap().values().filter(|s| s.online.load(Ordering::Relaxed)).map(|s| s.id.clone()).collect()
    }

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

    pub fn remove(&self, id: &str) {
        let s = self.students.lock().unwrap().remove(id);
        if let Some(s) = s {
            *s.tx.lock().unwrap() = None;
        }
        self.machines.lock().unwrap().remove(id);
        self.save_machines();
        let _ = self.app.emit("student_removed", id.to_string());
    }

    pub fn dismiss_pending(&self, id: &str) {
        self.pending.lock().unwrap().remove(id);
        let _ = self.app.emit("pending", self.pending());
    }

    pub fn set_seat(&self, id: &str, seat: Option<(u32, u32)>) {
        {
            let mut m = self.machines.lock().unwrap();
            // ghế đã có máy khác thì đổi chỗ hai máy
            if let Some(seat) = seat {
                let old = m.get(id).and_then(|x| x.seat);
                let other = m.iter().find(|(k, v)| k.as_str() != id && v.seat == Some(seat)).map(|(k, _)| k.clone());
                if let Some(o) = other {
                    if let Some(x) = m.get_mut(&o) {
                        x.seat = old;
                    }
                }
            }
            m.entry(id.to_string()).or_default().seat = seat;
        }
        self.save_machines();
        let list: Vec<Arc<Student>> = self.students.lock().unwrap().values().cloned().collect();
        for s in list {
            self.emit_student(&s);
        }
    }

    /// Xếp tự động các máy chưa có ghế theo tên
    pub fn auto_seats(&self, cols: u32) {
        let order: Vec<String> = self.students().into_iter().map(|s| s.id).collect();
        {
            let mut m = self.machines.lock().unwrap();
            for (i, id) in order.iter().enumerate() {
                let c = cols.max(1);
                m.entry(id.clone()).or_default().seat = Some((i as u32 % c, i as u32 / c));
            }
        }
        self.save_machines();
        let list: Vec<Arc<Student>> = self.students.lock().unwrap().values().cloned().collect();
        for s in list {
            self.emit_student(&s);
        }
    }

    pub fn set_label(&self, id: &str, label: &str) {
        self.machines.lock().unwrap().entry(id.to_string()).or_default().label = label.trim().chars().take(60).collect();
        self.save_machines();
        if let Some(s) = self.students.lock().unwrap().get(id).cloned() {
            self.emit_student(&s);
        }
    }

    // -------------------------------------------------------------- giới hạn (chặn web, ứng dụng, chế độ thi)
    fn signed_policy(&self, p: &Policy) -> Ctrl {
        let ts = now_ms();
        let sig = auth::sign_policy(&self.key, &serde_json::to_string(p).unwrap_or_default(), ts);
        Ctrl::SetPolicy { policy: p.clone(), ts, sig }
    }

    pub fn set_policy(&self, p: Policy) -> usize {
        *self.policy.write().unwrap() = p.clone();
        let _ = self.app.emit("policy", &p);
        self.send_ctrl(&self.online_ids(), &self.signed_policy(&p))
    }

    /// Máy học sinh tự bỏ giới hạn sau 3 phút không nhận lại (phòng khi Console bị tắt ngang),
    /// nên Console gửi lại mỗi phút.
    async fn refresh_policy(self: Arc<Self>) {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let p = self.policy();
            if p != Policy::default() {
                self.send_ctrl(&self.online_ids(), &self.signed_policy(&p));
            }
        }
    }

    /// Đẩy mật khẩu phòng mới xuống các máy đang nối (máy đã ghép vẫn dùng được, máy mới phải dùng mật khẩu mới)
    pub fn push_password(&self) -> usize {
        let pw = self.settings().room_password;
        let ts = now_ms();
        let sig = auth::sign_policy(&self.key, &format!("password:{pw}"), ts);
        self.send_ctrl(&self.online_ids(), &Ctrl::SetPairPassword { password: pw, ts, sig })
    }

    // -------------------------------------------------------------- bài tập
    pub fn assign(&self, ids: &[String], title: &str, text: &str) -> Result<Task> {
        let title = title.trim();
        if title.is_empty() {
            bail!("Hãy đặt tên bài tập");
        }
        let t = Task { id: format!("bt{}", now_ms()), title: title.into(), text: text.into(), created: now_ms(), collected: HashMap::new() };
        {
            let mut ts = self.tasks.lock().unwrap();
            ts.insert(0, t.clone());
            ts.truncate(100);
            let _ = write_json(&self.dir.join("tasks.json"), &*ts);
        }
        self.send_ctrl(ids, &Ctrl::Assign { task: t.id.clone(), title: t.title.clone(), text: t.text.clone() });
        let _ = self.app.emit("tasks", self.tasks());
        Ok(t)
    }

    pub fn collect(&self, ids: &[String], task: &str) -> Result<usize> {
        let t = {
            let mut ts = self.tasks.lock().unwrap();
            let t = ts.iter_mut().find(|t| t.id == task).ok_or_else(|| anyhow!("Không thấy bài tập"))?;
            for id in ids {
                t.collected.remove(id); // thu lại thì đếm lại từ đầu (file cũ bị ghi đè)
            }
            let t = t.clone();
            let _ = write_json(&self.dir.join("tasks.json"), &*ts);
            t
        };
        let _ = self.app.emit("tasks", self.tasks());
        std::fs::create_dir_all(self.task_dir(&t))?;
        Ok(self.send_ctrl(ids, &Ctrl::Collect { task: t.id, title: t.title }))
    }

    pub fn delete_task(&self, task: &str) {
        let mut ts = self.tasks.lock().unwrap();
        ts.retain(|t| t.id != task);
        let _ = write_json(&self.dir.join("tasks.json"), &*ts);
        drop(ts);
        let _ = self.app.emit("tasks", self.tasks());
    }

    pub fn task_dir_by_id(&self, task: &str) -> Option<PathBuf> {
        self.tasks.lock().unwrap().iter().find(|t| t.id == task).map(|t| self.task_dir(t))
    }

    fn task_dir(&self, t: &Task) -> PathBuf {
        self.out_dir.join("BaiNop").join(safe_part(&t.title))
    }

    fn collect_path(&self, task: &str, st: &Student, rel: &str) -> PathBuf {
        let base = self.tasks.lock().unwrap().iter().find(|t| t.id == task).map(|t| self.task_dir(t))
            .unwrap_or_else(|| self.out_dir.join("BaiNop").join(safe_part(task)));
        let info = st.info.read().unwrap().clone();
        let label = self.machines.lock().unwrap().get(&st.id).map(|m| m.label.clone()).unwrap_or_default();
        let who = if !label.is_empty() { format!("{} - {label}", info.name) }
                  else if info.user.is_empty() { info.name } else { format!("{} - {}", info.name, info.user) };
        base.join(safe_part(&who)).join(safe_rel(rel))
    }

    fn collected(&self, task: &str, st: &Student, path: &Path) {
        let n = {
            let mut ts = self.tasks.lock().unwrap();
            let Some(t) = ts.iter_mut().find(|t| t.id == task) else { return };
            let c = t.collected.entry(st.id.clone()).or_insert(0);
            *c += 1;
            let n = *c;
            let _ = write_json(&self.dir.join("tasks.json"), &*ts);
            n
        };
        let name = st.info.read().unwrap().name.clone();
        let _ = self.app.emit("collect", json!({"task": task, "student": st.id, "name": name, "done": false,
                                                 "files": n, "path": path.display().to_string()}));
    }

    // -------------------------------------------------------------- trắc nghiệm
    pub fn quiz_start(&self, ids: &[String], mut quiz: Quiz) -> Result<usize> {
        if quiz.questions.is_empty() {
            bail!("Bài trắc nghiệm chưa có câu hỏi");
        }
        for (i, q) in quiz.questions.iter().enumerate() {
            if q.q.trim().is_empty() || q.options.len() < 2 {
                bail!("Câu {} cần nội dung và ít nhất 2 đáp án", i + 1);
            }
            if q.answer.is_none_or(|a| a >= q.options.len()) {
                bail!("Câu {} chưa chọn đáp án đúng", i + 1);
            }
        }
        quiz.id = format!("tn{}", now_ms());
        let map = self.students.lock().unwrap();
        let results = ids.iter().filter_map(|id| map.get(id)).filter(|s| s.online.load(Ordering::Relaxed)).map(|s| {
            let i = s.info.read().unwrap();
            (s.id.clone(), QuizResult { answers: vec![-1; quiz.questions.len()], name: i.name.clone(), user: i.user.clone(), ..Default::default() })
        }).collect::<HashMap<_, _>>();
        drop(map);
        if results.is_empty() {
            bail!("Không có máy nào đang trực tuyến trong các máy đã chọn");
        }
        let targets: Vec<String> = results.keys().cloned().collect();
        let run = QuizRun { quiz: quiz.clone(), started: now_ms(), running: true, results };
        *self.quiz.lock().unwrap() = Some(run.clone());
        let _ = self.app.emit("quiz", &run);
        Ok(self.send_ctrl(&targets, &Ctrl::QuizStart { quiz: strip(&quiz) }))
    }

    pub fn quiz_stop(&self) {
        let ids = {
            let mut q = self.quiz.lock().unwrap();
            let Some(q) = q.as_mut() else { return };
            q.running = false;
            let _ = self.app.emit("quiz", &*q);
            let _ = self.save_quiz(q);
            (q.quiz.id.clone(), q.results.keys().cloned().collect::<Vec<_>>())
        };
        self.send_ctrl(&ids.1, &Ctrl::QuizStop { id: ids.0 });
    }

    fn quiz_answer(&self, st: &Student, id: &str, answers: Vec<i32>, user: String, done: bool) {
        let mut q = self.quiz.lock().unwrap();
        let Some(run) = q.as_mut().filter(|q| q.quiz.id == id) else { return };
        let correct = run.quiz.questions.iter().zip(&answers)
            .filter(|(q, a)| q.answer.map(|x| x as i32) == Some(**a)).count() as u32;
        let r = run.results.entry(st.id.clone()).or_default();
        if r.done {
            return; // đã nộp thì không sửa được
        }
        r.answers = answers;
        r.done = done;
        r.correct = correct;
        if !user.is_empty() {
            r.user = user;
        }
        r.name = st.info.read().unwrap().name.clone();
        let _ = self.app.emit("quiz", &*run);
        if done {
            let _ = self.save_quiz(run);
        }
    }

    fn save_quiz(&self, q: &QuizRun) -> Result<()> {
        let d = self.out_dir.join("TracNghiem");
        std::fs::create_dir_all(&d)?;
        let name = format!("{} - {}.csv", safe_part(&q.quiz.title), q.quiz.id);
        std::fs::write(d.join(name), quiz_csv(q))?;
        Ok(())
    }

    pub fn quiz_csv_path(&self) -> Option<PathBuf> {
        let q = self.quiz.lock().unwrap().clone()?;
        let _ = self.save_quiz(&q);
        Some(self.out_dir.join("TracNghiem").join(format!("{} - {}.csv", safe_part(&q.quiz.title), q.quiz.id)))
    }

    // -------------------------------------------------------------- gửi file
    pub fn send_files(self: &Arc<Self>, ids: Vec<String>, paths: Vec<String>) -> usize {
        let targets: Vec<Arc<Student>> = {
            let map = self.students.lock().unwrap();
            ids.iter().filter_map(|i| map.get(i).cloned()).filter(|s| s.online.load(Ordering::Relaxed)).collect()
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
                        let _ = me.app.emit("file_done", json!({"student": st.id, "id": fid, "ok": false,
                                                                 "path": p.display().to_string(), "error": format!("{e:#}")}));
                    }
                });
            }
        }
        n
    }

    async fn send_one(&self, st: &Student, p: &Path, fid: u64) -> Result<()> {
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
                me.notice(format!("Trình chiếu dừng: {e:#}"));
                me.stop_show();
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

/// Bỏ đáp án trước khi gửi đề xuống máy học sinh
fn strip(q: &Quiz) -> Quiz {
    let mut q = q.clone();
    for x in &mut q.questions {
        x.answer = None;
    }
    q
}

fn csv_cell(s: &str) -> String {
    if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

pub fn quiz_csv(q: &QuizRun) -> String {
    let n = q.quiz.questions.len();
    let mut out = String::from("\u{feff}Máy,Học sinh,Đã nộp,Số câu đúng,Điểm (thang 10)");
    for i in 0..n {
        out.push_str(&format!(",Câu {}", i + 1));
    }
    out.push('\n');
    let mut rows: Vec<&QuizResult> = q.results.values().collect();
    rows.sort_by(|a, b| natural(&a.name).cmp(&natural(&b.name)));
    for r in rows {
        let score = if n > 0 { r.correct as f32 * 10.0 / n as f32 } else { 0.0 };
        out.push_str(&format!("{},{},{},{},{:.2}", csv_cell(&r.name), csv_cell(&r.user), if r.done { "Có" } else { "Chưa" }, r.correct, score));
        for a in &r.answers {
            out.push(',');
            if *a >= 0 {
                out.push((b'A' + *a as u8) as char);
            }
        }
        out.push('\n');
    }
    out.push_str("Đáp án,,,,");
    for x in &q.quiz.questions {
        out.push(',');
        if let Some(a) = x.answer {
            out.push((b'A' + a as u8) as char);
        }
    }
    out.push('\n');
    out
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rel_paths_stay_inside() {
        assert_eq!(safe_rel("../../etc/passwd"), PathBuf::from("etc").join("passwd"));
        assert_eq!(safe_rel("/abs/x.txt"), PathBuf::from("abs").join("x.txt"));
        assert_eq!(safe_rel("a\\b.py"), PathBuf::from("a").join("b.py"));
        assert_eq!(safe_rel(".."), PathBuf::from("file"));
    }
}
