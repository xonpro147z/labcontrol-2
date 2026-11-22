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
}

#[derive(Clone, Default)]
pub struct UiMan(Arc<Mutex<State>>);

fn spawn_ui(mode: &str, text: &str, fullscreen: bool, piped: bool) -> Option<Child> {
    let exe = std::env::current_exe().ok()?;
    let mut c = Command::new(exe);
    c.args(["ui", mode, "--text", text]);
    if fullscreen {
        c.arg("--fullscreen");
    }
    c.stdin(if piped { Stdio::piped() } else { Stdio::null() });
    c.stdout(Stdio::null()).stderr(Stdio::null());
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
    pub fn start_watchdog(&self) {
        let me = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            me.reconcile();
        });
    }

    fn reconcile(&self) {
        let mut s = self.0.lock().unwrap();
        // khóa máy
        match s.want_lock.clone() {
            Some(msg) if !alive(&mut s.lock) => s.lock = spawn_ui("lock", &msg, true, false),
            None => kill(&mut s.lock),
            _ => {}
        }
        // trình chiếu
        match s.want_show.clone() {
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
