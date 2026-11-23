//! LabControl Agent 3 – chạy trên máy học sinh.
//!
//! Từ bản 3, máy học sinh **tự tìm và tự nối về máy giáo viên** trong mạng LAN (như iClassroom/NetSupport),
//! ghép lần đầu bằng **mật khẩu phòng**. Máy học sinh không mở cổng nào.
//!
//! Gồm hai phần:
//!  - Agent: chạy trong phiên đăng nhập của học sinh (chụp màn hình, điều khiển, màn hình khóa...).
//!  - Helper: dịch vụ quyền cao (giữ mật khẩu phòng, chặn web). Học sinh không tắt được.
//!
//! Khi giáo viên xem hoặc điều khiển, máy học sinh luôn hiện chỉ báo màu đỏ.
//! Agent không ghi phím và không lưu ảnh màn hình.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;
mod helper;
mod input;
mod link;
mod policy;
mod session;
mod sys;
mod tasks;
mod ui;
mod uiman;

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lc_core::proto::Ctrl;

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
    /// Cài cho máy học sinh (cần quyền quản trị). Mật khẩu phòng đặt trong Console > Cài đặt.
    Install {
        /// Mật khẩu phòng (giống trên máy giáo viên)
        #[arg(long)]
        password: String,
        /// Tên phòng (để trống nếu trong mạng chỉ có một phòng máy)
        #[arg(long, default_value = "")]
        room: String,
        /// Tên hiển thị của máy (mặc định: tên máy tính)
        #[arg(long, default_value = "")]
        name: String,
        /// Địa chỉ IP máy giáo viên, khi mạng chặn tự tìm máy (có thể ghi nhiều, cách nhau dấu phẩy)
        #[arg(long, default_value = "")]
        teacher_ip: String,
    },
    /// Đổi mật khẩu phòng trên máy này (cần quyền quản trị)
    SetPassword { password: String },
    /// Quên các máy giáo viên đã ghép (lần sau phải ghép lại bằng mật khẩu phòng)
    Forget,
    /// Gỡ Agent và Helper (giữ lại cấu hình)
    Uninstall,
    /// Xem cấu hình
    Status,
    /// Chạy Helper ở tiền cảnh (systemd gọi lệnh này trên Linux)
    Helper,
    #[command(hide = true)]
    HelperService,
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
    if !matches!(cli.cmd, None | Some(Cmd::Run) | Some(Cmd::Ui { .. }) | Some(Cmd::HelperService)) {
        attach_console();
    }
    if let Err(e) = real_main(cli) {
        eprintln!("Lỗi: {e:#}");
        tracing::error!("{e:#}");
        std::process::exit(1);
    }
}

fn real_main(cli: Cli) -> Result<()> {
    match cli.cmd.unwrap_or(Cmd::Run) {
        Cmd::Ui { mode, text, fullscreen } => {
            lc_core::capture::init_dpi();
            ui::run(&mode, &text, fullscreen)
        }
        Cmd::Run => run(),
        Cmd::Helper => {
            init_log("labcontrol-helper.log");
            tokio::runtime::Runtime::new()?.block_on(helper::serve())
        }
        Cmd::HelperService => {
            init_log("labcontrol-helper.log");
            #[cfg(windows)]
            {
                helper::service::run()
            }
            #[cfg(not(windows))]
            {
                tokio::runtime::Runtime::new()?.block_on(helper::serve())
            }
        }
        Cmd::Install { password, room, name, teacher_ip } => install(&password, &room, &name, &teacher_ip),
        Cmd::SetPassword { password } => {
            let mut c = config::load().unwrap_or_default();
            c.password = lc_core::pair::normalize(&password);
            config::save(&c)?;
            println!("Đã đổi mật khẩu phòng. Các máy giáo viên đã ghép vẫn dùng được.");
            Ok(())
        }
        Cmd::Forget => {
            let mut c = config::load()?;
            c.teachers.clear();
            config::save(&c)?;
            println!("Đã quên mọi máy giáo viên. Máy giáo viên sẽ ghép lại bằng mật khẩu phòng.");
            Ok(())
        }
        Cmd::Uninstall => uninstall(),
        Cmd::Status => {
            let c = config::load()?;
            println!("Tên máy:          {}", config::display_name(&c.name));
            println!("Phòng:            {}", if c.room.is_empty() { "(không đặt)" } else { &c.room });
            println!("Mật khẩu phòng:   {}", if c.password.is_empty() { "chưa đặt" } else { "đã đặt" });
            println!("Giáo viên đã ghép: {}", c.teachers.len());
            if !c.teacher_hosts.is_empty() {
                println!("IP máy giáo viên: {}", c.teacher_hosts.join(", "));
            }
            println!("Phiên bản:        {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn init_log(file: &str) {
    let filter = tracing_subscriber::EnvFilter::try_from_env("LABCONTROL_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let path = std::env::temp_dir().join(file);
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false).with_writer(std::sync::Mutex::new(f)).init(),
        Err(_) => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }
}

fn run() -> Result<()> {
    init_log("labcontrol-agent.log");
    lc_core::capture::init_dpi();
    let id = std::env::var("LABCONTROL_ID").unwrap_or_else(|_| config::hostname().to_lowercase());
    let (outbox, _) = tokio::sync::broadcast::channel::<Ctrl>(64);
    let ob = outbox.clone();
    let ui = uiman::UiMan::new(Arc::new(move |id, answers, done| {
        let _ = ob.send(Ctrl::QuizAnswer { id, answers, user: sys::username(), done });
    }));
    ui.start_watchdog();
    let guard = policy::Guard::default();
    let ob = outbox.clone();
    guard.start(move |name| {
        let _ = ob.send(Ctrl::Notice { text: format!("Đã chặn {name}") });
    });
    // chính sách hết hạn thì tắt dải "đang thi"
    {
        let (g, u) = (guard.clone(), ui.clone());
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            if u.exam() && !g.current().exam {
                u.set_exam(false);
            }
        });
    }
    let shared = Arc::new(session::Shared { id, input: input::Input::start(), ui, guard, outbox });
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(link::run(shared))
}

// ------------------------------------------------------------------ cài đặt
fn install(password: &str, room: &str, name: &str, teacher_ip: &str) -> Result<()> {
    let password = lc_core::pair::normalize(password);
    if password.chars().count() < 4 {
        anyhow::bail!("Mật khẩu phòng phải có ít nhất 4 ký tự");
    }
    let mut c = config::load().unwrap_or_default();
    c.password = password;
    c.room = room.trim().into();
    if !name.is_empty() {
        c.name = name.into();
    }
    let ips: Vec<String> = teacher_ip.split([',', ';', ' ']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    if !ips.is_empty() {
        c.teacher_hosts = ips;
    }
    config::save(&c)?;
    let exe = copy_self()?;
    install_helper(&exe)?;
    autostart(&exe)?;
    println!("Đã cài LabControl ({}).", exe.display());
    println!("Máy sẽ tự nối về máy giáo viên có cùng mật khẩu phòng sau khi học sinh đăng nhập.");
    println!("Hãy đăng xuất rồi đăng nhập lại (hoặc khởi động lại máy).");
    Ok(())
}

#[cfg(windows)]
fn install_helper(exe: &std::path::Path) -> Result<()> {
    // bản 2 mở cổng trên máy học sinh; bản 3 không cần nữa
    let _ = std::process::Command::new("netsh")
        .args(["advfirewall", "firewall", "delete", "rule", "name=LabControl Agent"]).output();
    helper::service::install(exe)
}

#[cfg(not(windows))]
fn install_helper(exe: &std::path::Path) -> Result<()> {
    let unit = format!(
        "[Unit]\nDescription=LabControl Helper\nAfter=network.target\n\n[Service]\nExecStart={} helper\nRestart=always\nRestartSec=3\n\n[Install]\nWantedBy=multi-user.target\n",
        exe.display()
    );
    std::fs::write("/etc/systemd/system/labcontrol-helper.service", unit).context("Cần chạy bằng sudo")?;
    let _ = std::process::Command::new("systemctl").arg("daemon-reload").status();
    let _ = std::process::Command::new("systemctl").args(["enable", "--now", "labcontrol-helper"]).status();
    let _ = std::process::Command::new("systemctl").args(["restart", "labcontrol-helper"]).status();
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

fn uninstall() -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_SET_VALUE};
        if let Ok(k) = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", KEY_SET_VALUE) {
            let _ = k.delete_value("LabControlAgent");
        }
        let _ = helper::service::uninstall();
    }
    #[cfg(not(windows))]
    {
        let _ = std::fs::remove_file("/etc/xdg/autostart/labcontrol-agent.desktop");
        let _ = std::process::Command::new("systemctl").args(["disable", "--now", "labcontrol-helper"]).status();
        let _ = std::fs::remove_file("/etc/systemd/system/labcontrol-helper.service");
    }
    let _ = helper::apply_hosts(&[]);
    println!("Đã gỡ LabControl. Agent dừng ở lần đăng nhập sau. Có thể xóa {} nếu muốn.", install_path().display());
    Ok(())
}
