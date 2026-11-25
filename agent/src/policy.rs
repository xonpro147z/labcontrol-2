//! Giới hạn trên máy học sinh: đóng công cụ hệ thống / ứng dụng bị cấm, chế độ thi.
//!
//! Việc chặn trang web do Helper (quyền cao) làm qua file hosts. Ở đây chỉ là phần chạy trong phiên
//! của học sinh: mỗi giây xem các tiến trình đang chạy và đóng những tiến trình bị cấm.
//! Chính sách tự hết hạn sau 3 phút nếu máy giáo viên không gửi lại (giáo viên tắt Console, mất mạng).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lc_core::proto::Policy;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

const TTL: Duration = Duration::from_secs(180);

#[cfg(windows)]
const TOOLS: &[&str] = &[
    "cmd.exe", "powershell.exe", "powershell_ise.exe", "pwsh.exe", "regedit.exe", "taskmgr.exe", "mmc.exe",
    "msconfig.exe", "windowsterminal.exe", "wt.exe", "gpedit.msc", "resmon.exe", "procexp.exe", "procexp64.exe",
    "processhacker.exe", "systeminformer.exe", "wsl.exe", "bash.exe",
];
#[cfg(not(windows))]
const TOOLS: &[&str] = &[
    "gnome-terminal", "gnome-terminal-server", "konsole", "xterm", "xfce4-terminal", "mate-terminal", "lxterminal",
    "tilix", "terminator", "kitty", "alacritty", "gnome-system-monitor", "ksysguard", "plasma-systemmonitor",
    "x-terminal-emulator", "qterminal",
];

/// Khi thi: đóng thêm trình duyệt và ứng dụng nhắn tin.
#[cfg(windows)]
const EXAM: &[&str] = &[
    "chrome.exe", "msedge.exe", "firefox.exe", "opera.exe", "opera_gx.exe", "brave.exe", "browser.exe", "vivaldi.exe",
    "zalo.exe", "telegram.exe", "discord.exe", "messenger.exe", "skype.exe", "ms-teams.exe", "teams.exe",
    "whatsapp.exe", "anydesk.exe", "teamviewer.exe", "ultraviewer_desktop.exe", "snippingtool.exe",
];
#[cfg(not(windows))]
const EXAM: &[&str] = &[
    "chrome", "google-chrome", "chromium", "chromium-browser", "firefox", "firefox-esr", "opera", "brave", "vivaldi-bin",
    "telegram-desktop", "discord", "skype", "zoom", "anydesk", "teamviewer",
];

#[derive(Default)]
struct State {
    policy: Policy,
    at: Option<Instant>,
    web_ok: bool,
    /// Tên đã đóng gần đây (để báo giáo viên, không báo liên tục)
    reported: HashMap<String, Instant>,
}

#[derive(Clone, Default)]
pub struct Guard(Arc<Mutex<State>>);

pub fn norm(n: &str) -> String {
    let n = n.trim().to_lowercase();
    let n = n.rsplit(['/', '\\']).next().unwrap_or("").to_string();
    n
}

fn matches(proc_name: &str, want: &str) -> bool {
    let p = norm(proc_name);
    let w = norm(want);
    if w.is_empty() {
        return false;
    }
    p == w || p.strip_suffix(".exe") == Some(w.as_str()) || w.strip_suffix(".exe") == Some(p.as_str())
}

impl Guard {
    pub fn set(&self, p: Policy, web_ok: bool) {
        let mut s = self.0.lock().unwrap();
        s.at = if p == Policy::default() { None } else { Some(Instant::now()) };
        s.policy = p;
        s.web_ok = web_ok;
    }

    /// Chính sách đang áp dụng (đã tính hết hạn)
    pub fn current(&self) -> Policy {
        let mut s = self.0.lock().unwrap();
        if matches!(s.at, Some(t) if t.elapsed() > TTL) {
            s.at = None;
            s.policy = Policy::default();
        }
        s.policy.clone()
    }

    pub fn web_blocked(&self) -> bool {
        let p = self.current();
        !p.block_web.is_empty() && self.0.lock().unwrap().web_ok
    }

    fn banned(&self) -> Vec<String> {
        let p = self.current();
        let mut v: Vec<String> = p.block_apps.iter().map(|a| norm(a)).filter(|a| !a.is_empty()).collect();
        if p.block_tools || p.exam {
            v.extend(TOOLS.iter().map(|s| s.to_string()));
        }
        if p.exam {
            v.extend(EXAM.iter().map(|s| s.to_string()));
        }
        v
    }

    /// Chạy nền: giữ máy ở trạng thái tắt tiếng khi giáo viên yêu cầu; bỏ lệnh thì bật lại tiếng (chỉ khi
    /// chính LabControl đã tắt).
    pub fn start_audio(&self, notify: impl Fn(String) + Send + 'static) {
        let me = self.clone();
        std::thread::spawn(move || {
            let mut we_muted = false;
            let mut warned = false;
            loop {
                std::thread::sleep(Duration::from_millis(1500));
                if me.current().mute {
                    if crate::audio::is_muted() != Some(true) {
                        match crate::audio::set_mute(true) {
                            Ok(()) => we_muted = true,
                            Err(e) if !warned => {
                                warned = true;
                                notify(format!("{e:#}"));
                            }
                            Err(_) => {}
                        }
                    }
                } else if we_muted {
                    let _ = crate::audio::set_mute(false);
                    we_muted = false;
                    warned = false;
                }
            }
        });
    }

    /// Chạy nền: mỗi giây đóng các tiến trình bị cấm. `notify` nhận tên tiến trình vừa đóng.
    pub fn start(&self, notify: impl Fn(String) + Send + 'static) {
        let me = self.clone();
        std::thread::spawn(move || {
            let mut sys = System::new_with_specifics(RefreshKind::nothing());
            let me_pid = sysinfo::get_current_pid().ok();
            loop {
                std::thread::sleep(Duration::from_millis(900));
                let banned = me.banned();
                if banned.is_empty() {
                    continue;
                }
                sys.refresh_processes_specifics(ProcessesToUpdate::All, true,
                                                ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet));
                for (pid, p) in sys.processes() {
                    if Some(*pid) == me_pid {
                        continue;
                    }
                    let name = p.name().to_string_lossy().to_string();
                    if name.to_lowercase().contains("labcontrol") {
                        continue;
                    }
                    if banned.iter().any(|b| matches(&name, b)) && p.kill() {
                        let mut s = me.0.lock().unwrap();
                        let key = norm(&name);
                        let fresh = s.reported.get(&key).map(|t| t.elapsed() > Duration::from_secs(60)).unwrap_or(true);
                        if fresh {
                            s.reported.insert(key, Instant::now());
                            drop(s);
                            notify(name);
                        }
                    }
                }
            }
        });
    }
}

/// Đóng mọi tiến trình có tên này (lệnh "Đóng ứng dụng" của giáo viên). Trả về số tiến trình đã đóng.
pub fn close_app(name: &str) -> usize {
    if norm(name).contains("labcontrol") {
        return 0;
    }
    let mut sys = System::new_with_specifics(RefreshKind::nothing());
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes().values().filter(|p| matches(&p.name().to_string_lossy(), name)).filter(|p| p.kill()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name_match() {
        assert!(matches("Chrome.exe", "chrome"));
        assert!(matches("chrome", "chrome.exe"));
        assert!(matches("ZALO.EXE", "zalo.exe"));
        assert!(!matches("chromium", "chrome"));
        assert!(!matches("x", ""));
    }
}
