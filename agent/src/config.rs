//! Cấu hình máy học sinh (chỉ quản trị viên/SYSTEM đọc-ghi được, vì có mật khẩu phòng).
//!
//! Agent chạy dưới tài khoản học sinh nên KHÔNG đọc trực tiếp file này: nó hỏi Helper (dịch vụ quyền cao)
//! qua cổng nội bộ 127.0.0.1. Khi chạy thử không có Helper, Agent đọc file trực tiếp nếu có quyền.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    /// Mật khẩu phòng dùng để ghép máy với máy giáo viên (rỗng = không ghép máy mới)
    #[serde(default)]
    pub password: String,
    /// Tên phòng: chỉ nối tới máy giáo viên cùng phòng (rỗng = mọi máy giáo viên)
    #[serde(default)]
    pub room: String,
    /// Tên hiển thị của máy; rỗng = tên máy tính
    #[serde(default)]
    pub name: String,
    /// Mã giáo viên (khóa công khai) đã ghép thành công
    #[serde(default)]
    pub teachers: Vec<String>,
    /// Địa chỉ máy giáo viên cố định (khi mạng chặn tìm máy tự động)
    #[serde(default)]
    pub teacher_hosts: Vec<String>,
}

/// Phần thông tin công khai Helper trả cho Agent (không có mật khẩu).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PublicConfig {
    pub room: String,
    pub name: String,
    pub teachers: Vec<String>,
    pub teacher_hosts: Vec<String>,
    pub has_password: bool,
}

impl Config {
    pub fn public(&self) -> PublicConfig {
        PublicConfig { room: self.room.clone(), name: self.name.clone(), teachers: self.teachers.clone(),
                       teacher_hosts: self.teacher_hosts.clone(), has_password: !self.password.is_empty() }
    }
}

pub fn dir() -> PathBuf {
    if let Ok(d) = std::env::var("LABCONTROL_AGENT_DIR") {
        return PathBuf::from(d);
    }
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into())).join("LabControl")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/etc/labcontrol")
    }
}

pub fn path() -> PathBuf {
    dir().join("agent.json")
}

pub fn load() -> Result<Config> {
    let p = path();
    let s = std::fs::read_to_string(&p).with_context(|| format!("Không đọc được {}", p.display()))?;
    Ok(serde_json::from_str(&s)?)
}

pub fn save(c: &Config) -> Result<()> {
    std::fs::create_dir_all(dir()).context("Không tạo được thư mục cấu hình (cần quyền quản trị)")?;
    let p = path();
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(c)?).context("Không ghi được cấu hình (cần quyền quản trị)")?;
    restrict(&tmp);
    std::fs::rename(&tmp, &p)?;
    restrict(&p);
    Ok(())
}

/// Chỉ quản trị viên và SYSTEM/root đọc được (file có mật khẩu phòng).
pub fn restrict(p: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::env::var("LABCONTROL_AGENT_DIR").is_err() {
            let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600));
        }
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("icacls").arg(p)
            .args(["/inheritance:r", "/grant:r", "*S-1-5-18:F", "*S-1-5-32-544:F"]).output();
    }
}

pub fn display_name(name: &str) -> String {
    if !name.trim().is_empty() {
        return name.trim().to_string();
    }
    hostname()
}

pub fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "may-hoc-sinh".into())
}

/// Nơi cài Agent (cũng là chương trình của Helper).
pub fn install_path() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into()))
            .join("LabControl").join("labcontrol-agent.exe")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/usr/local/bin/labcontrol-agent")
    }
}

/// Chép chương trình `src` vào nơi cài. Bản đang chạy được đổi tên thành .old trước
/// (Windows không cho ghi đè file đang chạy nhưng cho đổi tên).
pub fn install_binary(src: &std::path::Path) -> Result<PathBuf> {
    let dest = install_path();
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d).context("Không tạo được thư mục cài đặt (cần quyền quản trị)")?;
    }
    if dest.exists() {
        let mut old = dest.with_extension("old");
        if old.exists() && std::fs::remove_file(&old).is_err() {
            // bản .old cũ vẫn đang chạy (vừa cập nhật lần trước): dùng tên khác
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            old = dest.with_extension(format!("old{t}"));
        }
        std::fs::rename(&dest, &old).context("Không đổi tên được bản cũ")?;
    }
    if let Err(e) = std::fs::copy(src, &dest) {
        // trả lại bản cũ để máy vẫn chạy được
        let _ = std::fs::rename(dest.with_extension("old"), &dest);
        return Err(anyhow::anyhow!("Không chép được Agent vào thư mục cài đặt (cần quyền quản trị): {e}"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }
    Ok(dest)
}
