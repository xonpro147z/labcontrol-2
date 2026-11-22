//! LabControl Console 2 – máy giáo viên. Kết nối thẳng tới Agent trên các máy học sinh trong mạng LAN
//! (giống NetSupport School), không cần máy chủ.

mod hub;

use std::sync::Arc;

use hub::{Hub, Settings, StudentDto};
use lc_core::proto::{Ctrl, InputEv};
use serde::Serialize;
use tauri::{Manager, State};

type H<'a> = State<'a, Arc<Hub>>;

#[derive(Serialize)]
struct AppState {
    teacher_code: String,
    settings: Settings,
    students: Vec<StudentDto>,
    showing: bool,
}

#[tauri::command]
fn get_state(h: H<'_>) -> AppState {
    AppState { teacher_code: h.teacher_code(), settings: h.settings(), students: h.students(), showing: h.showing() }
}

#[tauri::command]
fn set_settings(h: H<'_>, settings: Settings) -> Result<(), String> {
    h.set_settings(settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_manual(h: H<'_>, addr: String) -> Result<(), String> {
    h.add_manual(&addr).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_student(h: H<'_>, id: String) {
    h.remove(&id);
}

/// Lệnh lớp học gửi tới nhiều máy. Chỉ chấp nhận các lệnh giáo viên được phép gửi.
#[tauri::command]
fn send_ctrl(h: H<'_>, ids: Vec<String>, msg: Ctrl) -> Result<usize, String> {
    match msg {
        Ctrl::Lock { .. } | Ctrl::Unlock | Ctrl::Message { .. } | Ctrl::OpenUrl { .. } | Ctrl::Power { .. }
        | Ctrl::StreamStart { .. } | Ctrl::StreamStop => Ok(h.send_ctrl(&ids, &msg)),
        _ => Err("Lệnh không được phép".into()),
    }
}

#[tauri::command]
fn input(h: H<'_>, id: String, ev: InputEv) {
    h.send_ctrl(&[id], &Ctrl::Input { ev });
}

#[tauri::command]
fn send_files(h: H<'_>, ids: Vec<String>, paths: Vec<String>) -> usize {
    h.send_files(ids, paths)
}

#[tauri::command]
fn start_show(h: H<'_>, ids: Vec<String>, fullscreen: bool) -> Result<(), String> {
    h.start_show(ids, fullscreen).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_show(h: H<'_>) {
    h.stop_show();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    lc_core::capture::init_dpi();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let hub = Hub::new(app.handle().clone(), dir)?;
            hub.clone().start();
            app.manage(hub);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_state, set_settings, add_manual, remove_student, send_ctrl, input,
                                                 send_files, start_show, stop_show])
        .run(tauri::generate_context!())
        .expect("không khởi động được LabControl");
}
