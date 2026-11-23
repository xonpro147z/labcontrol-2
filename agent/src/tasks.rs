//! Giao bài và thu bài.
//!
//! Giao bài: tạo thư mục "Desktop/BaiLam/<tên bài>", ghi đề bài vào DE-BAI.txt, mở thư mục cho học sinh.
//! Thu bài: gửi mọi file trong thư mục đó (trừ đề bài) về máy giáo viên.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub const DE_BAI: &str = "DE-BAI.txt";
const MAX_FILES: usize = 300;
const MAX_TOTAL: u64 = 500 << 20;

pub fn desktop() -> PathBuf {
    dirs::desktop_dir().or_else(dirs::home_dir).unwrap_or_else(std::env::temp_dir)
}

pub fn safe_name(n: &str) -> String {
    let base = n.replace('\\', "/");
    let base = base.rsplit('/').next().unwrap_or("file");
    let cleaned: String = base.chars().map(|c| if "<>:\"|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    if cleaned.is_empty() { "file".into() } else { cleaned.chars().take(120).collect() }
}

pub fn unique(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (1..).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).unwrap()
}

fn index_path() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join("LabControl").join("bai-tap.json")
}

fn load_index() -> HashMap<String, PathBuf> {
    std::fs::read(index_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_index(m: &HashMap<String, PathBuf>) {
    let p = index_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(p, serde_json::to_vec(m).unwrap_or_default());
}

pub fn task_dir(task: &str, title: &str) -> PathBuf {
    load_index().get(task).cloned()
        .unwrap_or_else(|| desktop().join("BaiLam").join(safe_name(if title.is_empty() { task } else { title })))
}

/// Tạo thư mục bài làm, ghi đề bài. Trả về đường dẫn thư mục.
pub fn assign(task: &str, title: &str, text: &str) -> Result<PathBuf> {
    let dir = desktop().join("BaiLam").join(safe_name(if title.is_empty() { task } else { title }));
    std::fs::create_dir_all(&dir)?;
    let mut body = String::new();
    body.push_str(title);
    body.push_str("\r\n\r\n");
    body.push_str(&text.replace("\r\n", "\n").replace('\n', "\r\n"));
    body.push_str("\r\n\r\n-- Lưu bài làm của em vào thư mục này. Giáo viên sẽ thu bài từ đây. --\r\n");
    std::fs::write(dir.join(DE_BAI), body)?;
    let mut m = load_index();
    m.insert(task.to_string(), dir.clone());
    save_index(&m);
    let _ = open::that_detached(&dir);
    Ok(dir)
}

/// Danh sách file cần nộp: (đường dẫn tương đối, đường dẫn đầy đủ, kích thước)
pub fn collect_list(dir: &Path) -> Result<Vec<(String, PathBuf, u64)>> {
    let mut out = Vec::new();
    let mut total = 0u64;
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() && depth < 4 {
                stack.push((p, depth + 1));
            } else if ft.is_file() {
                let rel = p.strip_prefix(dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                if rel == DE_BAI || rel.ends_with(".dangnhan") || rel.starts_with("~$") {
                    continue;
                }
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                total += size;
                if out.len() >= MAX_FILES || total > MAX_TOTAL {
                    anyhow::bail!("Thư mục bài làm quá lớn (tối đa {MAX_FILES} file, 500 MB)");
                }
                out.push((rel, p, size));
            }
        }
    }
    out.sort();
    Ok(out)
}
