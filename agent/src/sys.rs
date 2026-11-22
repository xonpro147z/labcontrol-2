//! Thông tin phiên làm việc: người dùng, hệ điều hành, cửa sổ đang dùng; lệnh tắt máy.

pub fn username() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default()
}

pub fn os_name() -> String {
    #[cfg(windows)]
    {
        "Windows".into()
    }
    #[cfg(not(windows))]
    {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| s.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=").map(|v| v.trim_matches('"').to_string())))
            .unwrap_or_else(|| "Linux".into())
    }
}

/// (ứng dụng, tiêu đề cửa sổ) đang ở trên cùng
#[cfg(windows)]
pub fn active_window() -> (String, String) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    unsafe {
        let h = GetForegroundWindow();
        if h.is_null() {
            return (String::new(), String::new());
        }
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
        let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, &mut pid);
        let mut app = String::new();
        let ph = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !ph.is_null() {
            let mut pbuf = [0u16; 520];
            let mut len = pbuf.len() as u32;
            if QueryFullProcessImageNameW(ph, 0, pbuf.as_mut_ptr(), &mut len) != 0 {
                let full = String::from_utf16_lossy(&pbuf[..len as usize]);
                app = full.rsplit('\\').next().unwrap_or("").to_string();
            }
            CloseHandle(ph);
        }
        (app, title)
    }
}

#[cfg(target_os = "linux")]
pub fn active_window() -> (String, String) {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    let Ok((conn, n)) = x11rb::connect(None) else { return (String::new(), String::new()) };
    let root = conn.setup().roots[n].root;
    let atom = |name: &str| conn.intern_atom(false, name.as_bytes()).ok().and_then(|c| c.reply().ok()).map(|r| r.atom);
    let (Some(active), Some(wm_name), Some(utf8), Some(wm_pid)) =
        (atom("_NET_ACTIVE_WINDOW"), atom("_NET_WM_NAME"), atom("UTF8_STRING"), atom("_NET_WM_PID"))
    else {
        return (String::new(), String::new());
    };
    let win = conn.get_property(false, root, active, AtomEnum::WINDOW, 0, 1).ok()
        .and_then(|c| c.reply().ok()).and_then(|r| r.value32().and_then(|mut v| v.next()));
    let Some(win) = win.filter(|w| *w != 0) else { return (String::new(), String::new()) };
    let title = conn.get_property(false, win, wm_name, utf8, 0, 1024).ok().and_then(|c| c.reply().ok())
        .map(|r| String::from_utf8_lossy(&r.value).to_string()).unwrap_or_default();
    let app = conn.get_property(false, win, wm_pid, AtomEnum::CARDINAL, 0, 1).ok().and_then(|c| c.reply().ok())
        .and_then(|r| r.value32().and_then(|mut v| v.next()))
        .and_then(|pid| std::fs::read_to_string(format!("/proc/{pid}/comm")).ok())
        .map(|s| s.trim().to_string()).unwrap_or_default();
    (app, title)
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn active_window() -> (String, String) {
    (String::new(), String::new())
}

pub fn power(action: &str) {
    #[cfg(windows)]
    let cmd: Option<(&str, Vec<&str>)> = match action {
        "shutdown" => Some(("shutdown", vec!["/s", "/t", "5"])),
        "restart" => Some(("shutdown", vec!["/r", "/t", "5"])),
        "logoff" => Some(("shutdown", vec!["/l"])),
        _ => None,
    };
    #[cfg(not(windows))]
    let user = username();
    #[cfg(not(windows))]
    let cmd: Option<(&str, Vec<&str>)> = match action {
        "shutdown" => Some(("systemctl", vec!["poweroff"])),
        "restart" => Some(("systemctl", vec!["reboot"])),
        "logoff" => Some(("loginctl", vec!["terminate-user", user.as_str()])),
        _ => None,
    };
    if let Some((p, a)) = cmd {
        let _ = std::process::Command::new(p).args(a).spawn();
    }
}
