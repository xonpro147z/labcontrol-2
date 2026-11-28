// Ẩn cửa sổ console trên Windows bản phát hành
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    labcontrol_console_lib::run()
}
