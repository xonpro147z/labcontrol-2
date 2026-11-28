//! LabControl Console 3 – máy giáo viên. Máy học sinh tự tìm và nối về máy này trong mạng LAN,
//! ghép lần đầu bằng mật khẩu phòng. Không cần máy chủ.

mod hub;

use std::sync::Arc;

use hub::{Hub, Pending, QuizRun, Settings, StudentDto, Task, UpdateInfo};
use lc_core::proto::{Ctrl, InputEv, Policy, Quiz};
use serde::Serialize;
use tauri::{Manager, State};

type H<'a> = State<'a, Arc<Hub>>;

#[derive(Serialize)]
struct AppState {
    teacher_code: String,
    settings: Settings,
    students: Vec<StudentDto>,
    pending: Vec<Pending>,
    showing: bool,
    policy: Policy,
    overrides: std::collections::HashMap<String, Policy>,
    tasks: Vec<Task>,
    quiz: Option<QuizRun>,
    listen_error: String,
    out_dir: String,
    addresses: Vec<String>,
    version: String,
    update: Option<UpdateInfo>,
    /// Hệ điều hành có sẵn file Agent đi kèm để cập nhật máy học sinh
    bundled: Vec<String>,
}

/// Địa chỉ IPv4 của máy giáo viên (để học sinh/kỹ thuật viên nhập tay khi mạng chặn tìm máy)
fn local_addresses() -> Vec<String> {
    let mut v = Vec::new();
    // mẹo: "nối" UDP tới một địa chỉ ngoài để biết card mạng chính (không gửi gói nào)
    if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if s.connect("10.255.255.255:9").is_ok() {
            if let Ok(a) = s.local_addr() {
                v.push(a.ip().to_string());
            }
        }
    }
    v
}

#[tauri::command]
fn get_state(h: H<'_>) -> AppState {
    AppState {
        teacher_code: h.teacher_code(), settings: h.settings(), students: h.students(), pending: h.pending(),
        showing: h.showing(), policy: h.policy(), overrides: h.overrides(), tasks: h.tasks(), quiz: h.quiz(), listen_error: h.listen_error(),
        out_dir: h.out_dir().display().to_string(), addresses: local_addresses(), version: env!("CARGO_PKG_VERSION").into(),
        update: h.latest(), bundled: h.bundled_platforms(),
    }
}

#[tauri::command]
fn set_settings(h: H<'_>, settings: Settings) -> Result<(), String> {
    h.set_settings(settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn push_password(h: H<'_>) -> usize {
    h.push_password()
}

#[tauri::command]
fn remove_student(h: H<'_>, id: String) {
    h.remove(&id);
}

#[tauri::command]
fn dismiss_pending(h: H<'_>, id: String) {
    h.dismiss_pending(&id);
}

#[tauri::command]
fn set_seat(h: H<'_>, id: String, seat: Option<(u32, u32)>) {
    h.set_seat(&id, seat);
}

#[tauri::command]
fn auto_seats(h: H<'_>, cols: u32) {
    h.auto_seats(cols);
}

#[tauri::command]
fn set_label(h: H<'_>, id: String, label: String) {
    h.set_label(&id, &label);
}

/// Lệnh lớp học gửi tới nhiều máy. Chỉ chấp nhận các lệnh giáo viên được phép gửi.
#[tauri::command]
fn send_ctrl(h: H<'_>, ids: Vec<String>, msg: Ctrl) -> Result<usize, String> {
    match msg {
        Ctrl::Lock { .. } | Ctrl::Unlock | Ctrl::Message { .. } | Ctrl::OpenUrl { .. } | Ctrl::Power { .. }
        | Ctrl::StreamStart { .. } | Ctrl::StreamStop | Ctrl::CloseApp { .. } | Ctrl::RunApp { .. } => Ok(h.send_ctrl(&ids, &msg)),
        _ => Err("Lệnh không được phép".into()),
    }
}

#[tauri::command]
fn input(h: H<'_>, id: String, ev: InputEv) {
    h.send_ctrl(&[id], &Ctrl::Input { ev });
}

#[tauri::command]
fn set_policy(h: H<'_>, policy: Policy, ids: Option<Vec<String>>) -> usize {
    h.set_policy(policy, ids)
}

#[tauri::command]
fn clear_override(h: H<'_>, ids: Vec<String>) -> usize {
    h.clear_override(ids)
}

#[tauri::command]
fn send_files(h: H<'_>, ids: Vec<String>, paths: Vec<String>) -> Result<hub::SendBatch, String> {
    h.send_files(ids, paths).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_show(h: H<'_>, ids: Vec<String>, fullscreen: bool) -> Result<(), String> {
    h.start_show(ids, fullscreen).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_show(h: H<'_>) {
    h.stop_show();
}

#[tauri::command]
fn assign(h: H<'_>, ids: Vec<String>, title: String, text: String) -> Result<Task, String> {
    h.assign(&ids, &title, &text).map_err(|e| e.to_string())
}

#[tauri::command]
fn collect(h: H<'_>, ids: Vec<String>, task: String) -> Result<usize, String> {
    h.collect(&ids, &task).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(h: H<'_>, task: String) {
    h.delete_task(&task);
}

#[tauri::command]
fn open_task_dir(h: H<'_>, task: String) -> Result<(), String> {
    let d = h.task_dir_by_id(&task).ok_or("Không thấy bài tập")?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    open_path(&d)
}

#[tauri::command]
fn quiz_start(h: H<'_>, ids: Vec<String>, quiz: Quiz) -> Result<usize, String> {
    h.quiz_start(&ids, quiz).map_err(|e| e.to_string())
}

#[tauri::command]
fn quiz_stop(h: H<'_>) {
    h.quiz_stop();
}

#[tauri::command]
async fn check_update(h: H<'_>) -> Result<UpdateInfo, String> {
    let h = h.inner().clone();
    h.check_update().await.map_err(|e| e.to_string())
}

#[tauri::command]
fn push_update(h: H<'_>, ids: Vec<String>, file: Option<String>) -> Result<usize, String> {
    h.push_update(&ids, file).map_err(|e| e.to_string())
}

/// Chỉ mở trang của dự án trên GitHub
#[tauri::command]
fn open_release(url: String) -> Result<(), String> {
    if !url.starts_with(&format!("https://github.com/{}/", hub::REPO)) {
        return Err("Địa chỉ không hợp lệ".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_quiz_csv(h: H<'_>) -> Result<(), String> {
    let p = h.quiz_csv_path().ok_or("Chưa có bài trắc nghiệm")?;
    open_path(p.parent().unwrap_or(&p))
}

fn open_path(p: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(p).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(p).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(p).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    lc_core::capture::init_dpi();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let out = app.path().document_dir().or_else(|_| app.path().home_dir())?.join("LabControl");
            let res = app.path().resource_dir().ok();
            let _ = std::fs::remove_file(dir.join("lock-media.lcm")); // ảnh nền khóa của bản 3.3.0–3.3.1 (đã bỏ)
            let hub = Hub::new(app.handle().clone(), dir, out, res)?;
            hub.clone().start();
            app.manage(hub);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state, set_settings, push_password, remove_student, dismiss_pending, set_seat, auto_seats, set_label,
            send_ctrl, input, set_policy, clear_override, send_files, start_show, stop_show, assign, collect, delete_task, open_task_dir,
            quiz_start, quiz_stop, open_quiz_csv, check_update, push_update, open_release
        ])
        .run(tauri::generate_context!())
        .expect("không khởi động được LabControl");
}
