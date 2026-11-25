//! Ngôn ngữ hiển thị trên máy học sinh. Máy giáo viên gửi lệnh SetLang khi máy học sinh nối vào;
//! lựa chọn được lưu lại để lần khởi động sau (và các cửa sổ con) dùng luôn.

use std::sync::atomic::{AtomicBool, Ordering};

static EN: AtomicBool = AtomicBool::new(false);

fn file() -> std::path::PathBuf {
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("LabControl").join("lang")
}

/// Đọc ngôn ngữ đã lưu (hoặc biến môi trường LABCONTROL_LANG do Agent truyền cho cửa sổ con)
pub fn init() {
    let l = std::env::var("LABCONTROL_LANG").ok().or_else(|| std::fs::read_to_string(file()).ok()).unwrap_or_default();
    EN.store(l.trim() == "en", Ordering::Relaxed);
}

pub fn set(lang: &str) {
    let en = lang == "en";
    if EN.swap(en, Ordering::Relaxed) != en || !file().exists() {
        let f = file();
        if let Some(d) = f.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(f, if en { "en" } else { "vi" });
    }
}

pub fn en() -> bool {
    EN.load(Ordering::Relaxed)
}

pub fn code() -> &'static str {
    if en() { "en" } else { "vi" }
}

/// l!("tiếng Việt {x}", "English {x}", ...)
macro_rules! l {
    ($vi:literal, $en:literal $(, $a:expr)* $(,)?) => {
        if crate::i18n::en() { format!($en $(, $a)*) } else { format!($vi $(, $a)*) }
    };
}
