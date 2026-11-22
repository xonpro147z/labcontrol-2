//! LabControl Agent 2 – chạy trên máy học sinh, theo mô hình NetSupport: máy giáo viên kết nối thẳng
//! tới đây trong mạng LAN, không cần máy chủ.
//!
//! Agent chạy trong phiên đăng nhập của học sinh (tự khởi động khi đăng nhập) để chụp được màn hình,
//! điều khiển chuột/bàn phím và hiện màn hình khóa. Khi giáo viên xem hoặc điều khiển, máy học sinh luôn
//! hiện chỉ báo màu đỏ. Agent không ghi phím và không lưu ảnh màn hình.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;
mod input;
mod session;
mod sys;
mod ui;
mod uiman;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use lc_core::{auth, tls, AGENT_PORT, BEACON_PORT};

#[derive(Parser)]
#[command(name = "labcontrol-agent", version,
          about = "LabControl Agent – phần mềm tự do theo giấy phép AGPL-3.0-or-later, không kèm bảo đảm.")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Chạy Agent (mặc định; được gọi tự động khi học sinh đăng nhập)
    Run,
    /// Cài Agent cho máy này (cần quyền quản trị). Lấy mã giáo viên trong Console > Cài đặt.
    Install {
        #[arg(long)]
        teacher: String,
        #[arg(long, default_value = "")]
        room: String,
        #[arg(long, default_value = "")]
        name: String,
    },
    /// Thêm mã một giáo viên khác được phép điều khiển máy này
    AddTeacher { code: String },
    /// Gỡ Agent (giữ lại cấu hình)
    Uninstall,
    /// Xem cấu hình
    Status,
    #[command(hide = true)]
    Ui {
        mode: String,
        #[arg(long, default_value = "")]
        text: String,
        #[arg(long)]
        fullscreen: bool,
    },
}

fn attach_console() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS);
    }
}

fn main() {
    let cli = Cli::parse();
    if !matches!(cli.cmd, None | Some(Cmd::Run) | Some(Cmd::Ui { .. })) {
        attach_console();
    }
    if let Err(e) = real_main(cli) {
        eprintln!("Lỗi: {e:#}");
        tracing::error!("{e:#}");
        std::process::exit(1);
    }
}

fn real_main(cli: Cli) -> Result<()> {
    lc_core::capture::init_dpi();
    match cli.cmd.unwrap_or(Cmd::Run) {
        Cmd::Ui { mode, text, fullscreen } => ui::run(&mode, &text, fullscreen),
        Cmd::Run => run(),
        Cmd::Install { teacher, room, name } => install(&teacher, &room, &name),
        Cmd::AddTeacher { code } => {
            auth::parse_teacher_code(&code)?;
            let mut c = config::load()?;
            if !c.teachers.iter().any(|t| t.trim() == code.trim()) {
                c.teachers.push(code.trim().into());
            }
            config::save(&c)?;
            println!("Đã thêm. Máy này nhận {} mã giáo viên. Agent áp dụng ở lần khởi động sau.", c.teachers.len());
            Ok(())
        }
        Cmd::Uninstall => uninstall(),
        Cmd::Status => {
            let c = config::load()?;
            println!("Tên máy:      {}", config::display_name(&c));
            println!("Phòng:        {}", if c.room.is_empty() { "(không đặt)" } else { &c.room });
            println!("Số giáo viên: {}", c.teachers.len());
            println!("Phiên bản:    {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn init_log() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("LABCONTROL_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let path = std::env::temp_dir().join("labcontrol-agent.log");
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false).with_writer(std::sync::Mutex::new(f)).init(),
        Err(_) => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }
}

fn run() -> Result<()> {
    init_log();
    let cfg = config::load()?;
    if cfg.teachers.is_empty() {
        bail!("Chưa có mã giáo viên nào trong cấu hình");
    }
    let port = std::env::var("LABCONTROL_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(AGENT_PORT);
    let id = format!("{}:{}", config::hostname(), port);
    let (srv, fp) = tls::server_config(&config::hostname())?;
    let ui = uiman::UiMan::default();
    ui.start_watchdog();
    let shared = Arc::new(session::Shared {
        cfg,
        id,
        acceptor: tokio_rustls::TlsAcceptor::from(srv),
        fp,
        input: input::Input::start(),
        ui,
        port,
    });
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let sh = shared.clone();
        tokio::spawn(beacon(sh));
        session::serve(shared).await
    })
}

/// Báo "tôi ở đây" mỗi 3 giây để Console tự tìm thấy máy này.
async fn beacon(sh: Arc<session::Shared>) {
    let Ok(sock) = tokio::net::UdpSocket::bind("0.0.0.0:0").await else { return };
    let _ = sock.set_broadcast(true);
    let targets: Vec<String> = std::env::var("LABCONTROL_BEACON_TO").map(|s| vec![s])
        .unwrap_or_else(|_| vec![format!("255.255.255.255:{BEACON_PORT}")]);
    loop {
        let msg = serde_json::json!({
            "lc": lc_core::PROTO_VERSION, "id": sh.id, "name": config::display_name(&sh.cfg),
            "room": sh.cfg.room, "port": sh.port, "user": sys::username(),
        });
        for t in &targets {
            let _ = sock.send_to(msg.to_string().as_bytes(), t).await;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

// ------------------------------------------------------------------ cài đặt
fn install(teacher: &str, room: &str, name: &str) -> Result<()> {
    auth::parse_teacher_code(teacher)?;
    let mut c = config::load().unwrap_or_default();
    if !c.teachers.iter().any(|t| t.trim() == teacher.trim()) {
        c.teachers.push(teacher.trim().into());
    }
    if !room.is_empty() {
        c.room = room.into();
    }
    if !name.is_empty() {
        c.name = name.into();
    }
    config::save(&c)?;
    let exe = copy_self()?;
    autostart(&exe)?;
    firewall(&exe);
    println!("Đã cài LabControl Agent ({}).", exe.display());
    println!("Agent sẽ tự chạy mỗi khi có người đăng nhập. Hãy đăng xuất rồi đăng nhập lại (hoặc khởi động lại máy).");
    Ok(())
}

fn install_path() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        std::path::PathBuf::from(std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into()))
            .join("LabControl").join("labcontrol-agent.exe")
    }
    #[cfg(not(windows))]
    {
        std::path::PathBuf::from("/usr/local/bin/labcontrol-agent")
    }
}

fn copy_self() -> Result<std::path::PathBuf> {
    let me = std::env::current_exe()?;
    let dest = install_path();
    if me != dest {
        if let Some(d) = dest.parent() {
            std::fs::create_dir_all(d).context("Không tạo được thư mục cài đặt (cần quyền quản trị)")?;
        }
        // đang chạy bản cũ thì đổi tên trước khi chép đè (Windows khóa file đang chạy)
        if dest.exists() {
            let old = dest.with_extension("old");
            let _ = std::fs::remove_file(&old);
            let _ = std::fs::rename(&dest, &old);
        }
        std::fs::copy(&me, &dest).context("Không chép được Agent vào thư mục cài đặt (cần quyền quản trị)")?;
    }
    Ok(dest)
}

#[cfg(windows)]
fn autostart(exe: &std::path::Path) -> Result<()> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_SET_VALUE};
    let k = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", KEY_SET_VALUE)
        .context("Cần chạy bằng quyền quản trị (Run as administrator)")?;
    k.set_value("LabControlAgent", &format!("\"{}\" run", exe.display()))?;
    Ok(())
}

#[cfg(not(windows))]
fn autostart(exe: &std::path::Path) -> Result<()> {
    let desktop = format!(
        "[Desktop Entry]\nType=Application\nName=LabControl Agent\nExec={} run\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    std::fs::create_dir_all("/etc/xdg/autostart")?;
    std::fs::write("/etc/xdg/autostart/labcontrol-agent.desktop", desktop).context("Cần chạy bằng sudo")?;
    Ok(())
}

fn firewall(exe: &std::path::Path) {
    #[cfg(windows)]
    {
        let run = |a: &[&str]| std::process::Command::new("netsh").args(a).output();
        let _ = run(&["advfirewall", "firewall", "delete", "rule", "name=LabControl Agent"]);
        let _ = run(&["advfirewall", "firewall", "add", "rule", "name=LabControl Agent", "dir=in", "action=allow",
                      &format!("program={}", exe.display()), "enable=yes", "profile=any"]);
    }
    #[cfg(not(windows))]
    {
        let _ = exe;
        let active = std::process::Command::new("ufw").arg("status").output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("Status: active")).unwrap_or(false);
        if active {
            let _ = std::process::Command::new("ufw").args(["allow", &format!("{AGENT_PORT}/tcp")]).status();
        }
    }
}

fn uninstall() -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_SET_VALUE};
        if let Ok(k) = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", KEY_SET_VALUE) {
            let _ = k.delete_value("LabControlAgent");
        }
        let _ = std::process::Command::new("netsh")
            .args(["advfirewall", "firewall", "delete", "rule", "name=LabControl Agent"]).output();
    }
    #[cfg(not(windows))]
    {
        let _ = std::fs::remove_file("/etc/xdg/autostart/labcontrol-agent.desktop");
    }
    println!("Đã gỡ tự khởi động. Agent dừng ở lần đăng nhập sau. Có thể xóa {} nếu muốn.", install_path().display());
    Ok(())
}
