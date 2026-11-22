// Ẩn cửa sổ console trên Windows bản phát hành
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    labcontrol_console_lib::run()
}
