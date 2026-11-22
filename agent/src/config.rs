//! Cấu hình Agent: danh sách mã giáo viên (khóa công khai) được phép điều khiển máy, tên phòng.
//! File chỉ chứa khóa công khai nên để mọi người dùng đọc được (Agent chạy dưới tài khoản học sinh).

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    /// Mã giáo viên LCT-... được phép kết nối
    pub teachers: Vec<String>,
    /// Tên phòng (Console có thể lọc theo phòng)
    #[serde(default)]
    pub room: String,
    /// Tên hiển thị của máy; để trống = tên máy tính
    #[serde(default)]
    pub name: String,
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
    let s = std::fs::read_to_string(&p)
        .with_context(|| format!("Chưa cài đặt Agent (không thấy {}). Chạy: labcontrol-agent install --teacher LCT-...", p.display()))?;
    Ok(serde_json::from_str(&s)?)
}

pub fn save(c: &Config) -> Result<()> {
    std::fs::create_dir_all(dir()).context("Không tạo được thư mục cấu hình (cần quyền quản trị)")?;
    std::fs::write(path(), serde_json::to_string_pretty(c)?).context("Không ghi được cấu hình (cần quyền quản trị)")?;
    Ok(())
}

pub fn display_name(c: &Config) -> String {
    if !c.name.trim().is_empty() {
        return c.name.trim().to_string();
    }
    hostname()
}

pub fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "may-hoc-sinh".into())
}
