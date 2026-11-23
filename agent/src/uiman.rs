//! Quản lý các cửa sổ con (khóa, trình chiếu, chỉ báo). Nếu học sinh tắt cửa sổ khóa hoặc
//! cửa sổ trình chiếu bằng Task Manager, Agent tự mở lại trong vòng 1 giây.

use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    want_lock: Option<String>,
    want_show: Option<bool>, // Some(toàn màn hình?)
    viewers: usize,
    lock: Option<Child>,
    view: Option<Child>,
    view_tx: Option<SyncSender<Vec<u8>>>,
    badge: Option<Child>,
    want_exam: bool,
    exam: Option<Child>,
    quiz: Option<QuizRun>,
    /// Agent sắp khởi động lại (cập nhật): không mở lại cửa sổ nào nữa
    stopping: bool,
}

/// Bài trắc nghiệm đang làm. Cửa sổ bị tắt ngang thì mở lại với các câu đã chọn.
struct QuizRun {
    quiz: lc_core::proto::Quiz,
    answers: Vec<i32>,
    deadline: Option<std::time::Instant>,
    done: bool,
    child: Option<Child>,
}

/// Sự kiện từ cửa sổ trắc nghiệm gửi lên máy giáo viên
pub type QuizSink = Arc<dyn Fn(String, Vec<i32>, bool) + Send + Sync>;

#[derive(Clone)]
pub struct UiMan(Arc<Mutex<State>>, QuizSink);

fn spawn_ui(mode: &str, text: &str, fullscreen: bool, piped: bool) -> Option<Child> {
    let exe = std::env::current_exe().ok()?;
    let mut c = Command::new(exe);
    c.args(["ui", mode, "--text", text]);
    if fullscreen {
        c.arg("--fullscreen");
    }
    c.stdin(if piped { Stdio::piped() } else { Stdio::null() });
    c.stdout(if mode == "quiz" { Stdio::piped() } else { Stdio::null() }).stderr(Stdio::null());
    match c.spawn() {
        Ok(ch) => Some(ch),
        Err(e) => {
            tracing::warn!("Không mở được cửa sổ {mode}: {e}");
            None
        }
    }
}

fn alive(c: &mut Option<Child>) -> bool {
    match c {
        Some(ch) => matches!(ch.try_wait(), Ok(None)),
        None => false,
    }
}

fn kill(c: &mut Option<Child>) {
    if let Some(mut ch) = c.take() {
        let _ = ch.kill();
        let _ = ch.wait();
    }
}

impl UiMan {
    pub fn new(sink: QuizSink) -> Self {
        UiMan(Arc::new(Mutex::new(State::default())), sink)
    }

    pub fn start_watchdog(&self) {
        let me = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            me.reconcile();
        });
    }

    /// Đóng mọi cửa sổ con trước khi Agent thoát (để bản mới mở lại).
    pub fn shutdown(&self) {
        let mut s = self.0.lock().unwrap();
        s.stopping = true;
        s.view_tx = None;
        kill(&mut s.lock);
        kill(&mut s.view);
        kill(&mut s.badge);
        kill(&mut s.exam);
        if let Some(q) = &mut s.quiz {
            kill(&mut q.child);
        }
    }

    fn reconcile(&self) {
        let mut s = self.0.lock().unwrap();
        if s.stopping {
            return;
        }
        // khóa máy
        match s.want_lock.clone() {
            Some(msg) if !alive(&mut s.lock) => s.lock = spawn_ui("lock", &msg, true, false),
            None => kill(&mut s.lock),
            _ => {}
        }
        // trình chiếu
        match s.want_show.as_ref().copied() {
            Some(fs) if !alive(&mut s.view) => {
                kill(&mut s.view);
                s.view_tx = None;
                if let Some(mut ch) = spawn_ui("view", "", fs, true) {
                    if let Some(mut stdin) = ch.stdin.take() {
                        let (tx, rx) = sync_channel::<Vec<u8>>(6);
                        std::thread::spawn(move || {
                            while let Ok(pkt) = rx.recv() {
                                let len = (pkt.len() as u32).to_be_bytes();
                                if stdin.write_all(&len).and_then(|_| stdin.write_all(&pkt)).and_then(|_| stdin.flush()).is_err() {
                                    break;
                                }
                            }
                        });
                        s.view_tx = Some(tx);
                    }
                    s.view = Some(ch);
                }
            }
            None => {
                s.view_tx = None;
                kill(&mut s.view);
            }
            _ => {}
        }
        // chỉ báo đang được xem
        if s.viewers > 0 {
            if !alive(&mut s.badge) {
                s.badge = spawn_ui("badge", "", false, false);
            }
        } else {
            kill(&mut s.badge);
        }
        // dải "Đang thi"
        if s.want_exam {
            if !alive(&mut s.exam) {
                s.exam = spawn_ui("exam", "", false, false);
            }
        } else {
            kill(&mut s.exam);
        }
        // trắc nghiệm
        let mut finished: Option<(String, Vec<i32>)> = None;
        if let Some(q) = &mut s.quiz {
            if let Some(d) = q.deadline {
                if std::time::Instant::now() > d + std::time::Duration::from_secs(3) && !q.done {
                    q.done = true;
                    finished = Some((q.quiz.id.clone(), q.answers.clone()));
                }
            }
            if q.done {
                kill(&mut q.child);
            } else if !alive(&mut q.child) {
                q.child = self.spawn_quiz(q);
            }
        }
        if let Some((id, a)) = finished {
            drop(s);
            (self.1)(id, a, true);
        }
    }

    fn spawn_quiz(&self, q: &QuizRun) -> Option<Child> {
        let mut ch = spawn_ui("quiz", "", false, true)?;
        let left = q.deadline.map(|d| d.saturating_duration_since(std::time::Instant::now()).as_secs()).unwrap_or(0);
        let init = serde_json::json!({"quiz": q.quiz, "answers": q.answers, "seconds_left": left, "timed": q.deadline.is_some()});
        if let Some(mut stdin) = ch.stdin.take() {
            let _ = writeln!(stdin, "{init}");
            // giữ stdin mở: cửa sổ tự đóng khi Agent dừng
            std::thread::spawn(move || {
                let _keep = stdin;
                std::thread::sleep(std::time::Duration::from_secs(86400));
            });
        }
        if let Some(out) = ch.stdout.take() {
            let me = self.clone();
            let id = q.quiz.id.clone();
            std::thread::spawn(move || {
                use std::io::BufRead;
                for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                    let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
                    let answers: Vec<i32> = serde_json::from_value(v["answers"].clone()).unwrap_or_default();
                    let done = v["done"].as_bool().unwrap_or(false);
                    let mut s = me.0.lock().unwrap();
                    let Some(q) = s.quiz.as_mut().filter(|q| q.quiz.id == id && !q.done) else { break };
                    q.answers = answers.clone();
                    q.done = done;
                    drop(s);
                    (me.1)(id.clone(), answers, done);
                    if done {
                        break;
                    }
                }
            });
        }
        Some(ch)
    }

    pub fn quiz_start(&self, quiz: lc_core::proto::Quiz) {
        let mut s = self.0.lock().unwrap();
        if let Some(old) = &mut s.quiz {
            if old.quiz.id == quiz.id {
                return; // giáo viên gửi lại (nối lại mạng) – giữ bài đang làm
            }
            kill(&mut old.child);
        }
        let n = quiz.questions.len();
        let deadline = (quiz.minutes > 0).then(|| std::time::Instant::now() + std::time::Duration::from_secs(quiz.minutes as u64 * 60));
        s.quiz = Some(QuizRun { quiz, answers: vec![-1; n], deadline, done: false, child: None });
        drop(s);
        self.reconcile();
    }

    /// Giáo viên thu bài trắc nghiệm: nộp phần đã làm
    pub fn quiz_stop(&self, id: &str) {
        let mut s = self.0.lock().unwrap();
        let Some(mut q) = s.quiz.take().filter(|q| q.quiz.id == id) else { return };
        kill(&mut q.child);
        drop(s);
        if !q.done {
            (self.1)(q.quiz.id, q.answers, true);
        }
    }

    pub fn set_exam(&self, on: bool) {
        self.0.lock().unwrap().want_exam = on;
        self.reconcile();
    }

    pub fn exam(&self) -> bool {
        self.0.lock().unwrap().want_exam
    }

    pub fn lock(&self, msg: &str) {
        self.0.lock().unwrap().want_lock = Some(if msg.trim().is_empty() { "Hãy chú ý lên giáo viên".into() } else { msg.into() });
        self.reconcile();
    }

    pub fn unlock(&self) {
        self.0.lock().unwrap().want_lock = None;
        self.reconcile();
    }

    pub fn locked(&self) -> bool {
        self.0.lock().unwrap().want_lock.is_some()
    }

    pub fn show(&self, fullscreen: bool) {
        let mut s = self.0.lock().unwrap();
        if s.want_show != Some(fullscreen) {
            s.want_show = Some(fullscreen);
            s.view_tx = None;
            kill(&mut s.view);
        }
        drop(s);
        self.reconcile();
    }

    pub fn stop_show(&self) {
        self.0.lock().unwrap().want_show = None;
        self.reconcile();
    }

    pub fn showing(&self) -> bool {
        self.0.lock().unwrap().want_show.is_some()
    }

    /// Chuyển một gói ô ảnh tới cửa sổ trình chiếu; bỏ qua nếu cửa sổ đang bận (khung khóa kế tiếp sẽ bù lại).
    pub fn feed(&self, pkt: Vec<u8>) {
        let s = self.0.lock().unwrap();
        if let Some(tx) = &s.view_tx {
            if let Err(TrySendError::Disconnected(_)) = tx.try_send(pkt) {
                tracing::debug!("cửa sổ trình chiếu đã đóng");
            }
        }
    }

    pub fn viewer(&self, delta: i32) {
        {
            let mut s = self.0.lock().unwrap();
            s.viewers = (s.viewers as i32 + delta).max(0) as usize;
        }
        self.reconcile();
    }

    pub fn viewed(&self) -> bool {
        self.0.lock().unwrap().viewers > 0
    }

    pub fn message(&self, text: &str) {
        if let Some(mut ch) = spawn_ui("message", text, false, false) {
            std::thread::spawn(move || {
                let _ = ch.wait();
            });
        }
    }
}
