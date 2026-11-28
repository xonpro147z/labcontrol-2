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

/// Mở ứng dụng theo yêu cầu của giáo viên: tên chương trình ("notepad", "firefox"), đường dẫn,
/// hoặc dòng lệnh có tham số ("libreoffice --writer"). Đường dẫn có dấu cách tự được đặt trong ngoặc kép.
pub fn run_app(cmd: &str) -> std::io::Result<()> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Err(std::io::Error::other("chưa nhập ứng dụng"));
    }
    let line = if !cmd.starts_with('"') && std::path::Path::new(cmd).exists() { format!("\"{cmd}\"") } else { cmd.to_string() };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // "start" tìm được cả ứng dụng đăng ký trong App Paths (winword, excel, chrome...)
        std::process::Command::new("cmd").raw_arg(format!("/C start \"\" {line}"))
            .creation_flags(0x0800_0000).spawn()?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        // kiểm tra chương trình có trên máy không (bỏ qua khi đường dẫn nằm trong ngoặc kép)
        let check = if line.starts_with('"') {
            String::new()
        } else {
            let first = line.split_whitespace().next().unwrap_or("");
            format!("command -v '{first}' >/dev/null 2>&1 || [ -e '{first}' ] || exit 127; ")
        };
        let status = std::process::Command::new("sh").arg("-c")
            .arg(format!("{check}setsid {line} >/dev/null 2>&1 &"))
            .status()?;
        if status.code() == Some(127) {
            return Err(std::io::Error::other("không tìm thấy ứng dụng trên máy này"));
        }
        Ok(())
    }
}

/// Địa chỉ MAC các card mạng (ưu tiên card thật), để máy giáo viên mở máy bằng Wake-on-LAN
pub fn mac_addresses() -> Vec<String> {
    let (mut real, mut local): (Vec<String>, Vec<String>) = (vec![], vec![]);
    if let Ok(it) = mac_address::MacAddressIterator::new() {
        for m in it {
            let b = m.bytes();
            // bỏ 00:00:.. và địa chỉ multicast
            if b.iter().all(|x| *x == 0) || b[0] & 0x01 != 0 {
                continue;
            }
            let s = b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(":");
            // địa chỉ "locally administered" thường là card ảo: chỉ dùng khi không có card thật
            let list = if b[0] & 0x02 == 0 { &mut real } else { &mut local };
            if !list.contains(&s) {
                list.push(s);
            }
        }
    }
    let mut out = if real.is_empty() { local } else { real };
    out.truncate(8);
    out
}
