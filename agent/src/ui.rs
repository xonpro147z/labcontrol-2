//! Các cửa sổ hiện trên máy học sinh. Mỗi cửa sổ là một tiến trình con riêng
//! (`labcontrol-agent ui <loại>`), để tiến trình chính luôn nhẹ và không bị treo theo giao diện.
//!
//!  - lock:    màn hình khóa toàn màn hình, chặn bàn phím/chuột
//!  - message: tin nhắn của giáo viên
//!  - view:    xem trình chiếu màn hình giáo viên (dữ liệu ô ảnh đọc từ stdin)
//!  - badge:   chỉ báo "Giáo viên đang xem màn hình của bạn"
//!  - exam:    dải "Đang trong giờ kiểm tra"
//!  - quiz:    bài trắc nghiệm (đề đọc từ stdin, câu trả lời ghi ra stdout)

use std::io::Read;
use std::sync::{Arc, Mutex};

use eframe::egui;

const BG: egui::Color32 = egui::Color32::from_rgb(15, 23, 42);
// Màu nhận diện LabControl (xanh ngọc đậm)
const INDIGO: egui::Color32 = egui::Color32::from_rgb(13, 148, 136);
const INK: egui::Color32 = egui::Color32::from_rgb(15, 23, 42);

fn setup_fonts(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\arial.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        "/usr/share/fonts/opentype/noto/NotoSans-Regular.ttf",
    ];
    if let Some(bytes) = candidates.iter().find_map(|p| std::fs::read(p).ok()) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("sys".into(), Arc::new(egui::FontData::from_owned(bytes)));
        if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            f.insert(0, "sys".into());
        }
        ctx.set_fonts(fonts);
    }
}

fn screen_size() -> (f32, f32) {
    lc_core::capture::Capturer::new().map(|c| c.size()).map(|(w, h)| (w as f32, h as f32)).unwrap_or((1366.0, 768.0))
}

fn keep_open(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().close_requested()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    }
}

pub fn run(mode: &str, text: &str, fullscreen: bool) -> anyhow::Result<()> {
    let (sw, sh) = screen_size();
    let mut vp = egui::ViewportBuilder::default().with_title("LabControl").with_always_on_top();
    let app: Box<dyn FnOnce(&egui::Context) -> Box<dyn eframe::App>> = match mode {
        "lock" => {
            vp = vp.with_decorations(false).with_fullscreen(true).with_position([0.0, 0.0]).with_inner_size([sw, sh]);
            let t = text.to_string();
            Box::new(move |ctx| {
                block_input();
                let media = std::env::var_os("LABCONTROL_LOCK_MEDIA").and_then(|p| std::fs::read(p).ok())
                    .and_then(|d| lc_core::lockmedia::unpack(&d).ok())
                    .map(|(fps, frames)| Media { fps, frames, idx: usize::MAX, next: std::time::Instant::now(), tex: None });
                Box::new(Lock { text: t, ctx: ctx.clone(), media, focused: None })
            })
        }
        "message" => {
            let (w, h) = (520.0, 240.0);
            vp = vp.with_inner_size([w, h]).with_position([(sw - w) / 2.0, (sh - h) / 3.0]).with_resizable(false)
                .with_title(l!("Tin nhắn từ giáo viên", "Message from the teacher"));
            let t = text.to_string();
            Box::new(move |_| Box::new(Message { text: t }))
        }
        "view" => {
            vp = if fullscreen {
                vp.with_decorations(false).with_fullscreen(true).with_position([0.0, 0.0]).with_inner_size([sw, sh])
            } else {
                vp.with_inner_size([sw * 0.7, sh * 0.7]).with_title(l!("Giáo viên đang trình chiếu", "Teacher presentation"))
            };
            Box::new(move |ctx| {
                // trình chiếu toàn màn hình: học sinh không thoát ra màn hình chính được (giống màn hình khóa)
                if fullscreen {
                    block_input();
                }
                let shared: Arc<Mutex<Option<egui::ColorImage>>> = Default::default();
                let s2 = shared.clone();
                let c2 = ctx.clone();
                std::thread::spawn(move || read_tiles(s2, c2));
                Box::new(View { shared, tex: None, fullscreen })
            })
        }
        "exam" => {
            let (w, h) = (300.0, 34.0);
            vp = vp.with_decorations(false).with_inner_size([w, h]).with_position([(sw - w) / 2.0, 0.0])
                .with_mouse_passthrough(true).with_resizable(false);
            Box::new(|_| Box::new(Exam))
        }
        "quiz" => {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let init: serde_json::Value = serde_json::from_str(&line)?;
            let quiz: lc_core::proto::Quiz = serde_json::from_value(init["quiz"].clone())?;
            let mut answers: Vec<i32> = serde_json::from_value(init["answers"].clone()).unwrap_or_default();
            answers.resize(quiz.questions.len(), -1);
            let deadline = init["timed"].as_bool().unwrap_or(false)
                .then(|| std::time::Instant::now() + std::time::Duration::from_secs(init["seconds_left"].as_u64().unwrap_or(0)));
            let (w, h) = ((sw * 0.6).clamp(560.0, 900.0), (sh * 0.8).clamp(420.0, 900.0));
            vp = vp.with_inner_size([w, h]).with_position([(sw - w) / 2.0, (sh - h) / 2.0])
                .with_title(l!("Trắc nghiệm: {}", "Quiz: {}", quiz.title));
            Box::new(move |_| Box::new(QuizWin { quiz, answers, deadline, confirm: false, sent_done: false }))
        }
        _ => {
            let (w, h) = (360.0, 38.0);
            vp = vp.with_decorations(false).with_inner_size([w, h]).with_position([sw - w - 16.0, 16.0])
                .with_mouse_passthrough(true).with_resizable(false);
            Box::new(|_| Box::new(Badge))
        }
    };
    let opts = eframe::NativeOptions { viewport: vp, ..Default::default() };
    eframe::run_native("LabControl", opts, Box::new(move |cc| {
        setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        Ok(app(&cc.egui_ctx))
    }))
    .map_err(|e| anyhow::anyhow!("Không mở được cửa sổ: {e}"))
}

// ------------------------------------------------------------------ khóa máy
struct Lock {
    text: String,
    ctx: egui::Context,
    media: Option<Media>,
    focused: Option<std::time::Instant>,
}

/// Ảnh hoặc video (chuỗi khung JPEG) làm nền màn hình khóa
struct Media {
    fps: u16,
    frames: Vec<Vec<u8>>,
    idx: usize,
    next: std::time::Instant,
    tex: Option<egui::TextureHandle>,
}

impl Media {
    /// Chuyển khung khi tới giờ. Trả về thời gian chờ tới khung kế tiếp.
    fn step(&mut self, ctx: &egui::Context) -> Option<std::time::Duration> {
        let now = std::time::Instant::now();
        let still = self.fps == 0 || self.frames.len() < 2;
        if self.tex.is_none() || (!still && now >= self.next) {
            self.idx = if self.idx == usize::MAX { 0 } else { (self.idx + 1) % self.frames.len() };
            if let Ok((w, h, px)) = lc_core::img::decode_rgba(&self.frames[self.idx]) {
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px);
                match &mut self.tex {
                    Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                    None => self.tex = Some(ctx.load_texture("lock-bg", img, egui::TextureOptions::LINEAR)),
                }
            }
            let dt = std::time::Duration::from_secs_f32(1.0 / self.fps.clamp(1, 30) as f32);
            self.next = if now > self.next + dt { now + dt } else { self.next + dt };
        }
        (!still).then(|| self.next.saturating_duration_since(std::time::Instant::now()))
    }
}

/// Phủ kín khung (cắt bớt phần thừa), giống "object-fit: cover"
fn cover_uv(img: egui::Vec2, area: egui::Rect) -> egui::Rect {
    let (ia, aa) = (img.x / img.y.max(1.0), area.width() / area.height().max(1.0));
    if ia > aa {
        let w = aa / ia;
        egui::Rect::from_min_max(egui::pos2((1.0 - w) / 2.0, 0.0), egui::pos2((1.0 + w) / 2.0, 1.0))
    } else {
        let h = ia / aa;
        egui::Rect::from_min_max(egui::pos2(0.0, (1.0 - h) / 2.0), egui::pos2(1.0, (1.0 + h) / 2.0))
    }
}

impl eframe::App for Lock {
    fn ui(&mut self, root: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        keep_open(ctx);
        let wait = self.media.as_mut().and_then(|m| m.step(ctx));
        let bg = self.media.as_ref().and_then(|m| m.tex.as_ref()).map(|t| (t.id(), t.size_vec2()));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(BG)).show(root, |ui| {
            let full = ui.max_rect();
            if let Some((id, size)) = bg {
                let p = ui.painter();
                p.image(id, full, cover_uv(size, full), egui::Color32::WHITE);
                // làm tối phần dưới để chữ luôn đọc được
                let band = egui::Rect::from_min_max(egui::pos2(full.left(), (full.bottom() - 190.0).round()), full.max);
                // dải tối mờ dần từ trên xuống (mỗi hàng điểm ảnh một lớp, không lộ vạch)
                let fade = 110.0_f32;
                let top = (band.top() - fade).round();
                for k in 0..fade as i32 {
                    let y = top + k as f32;
                    let a = 150.0 * (k as f32 + 0.5) / fade;
                    p.rect_filled(egui::Rect::from_min_max(egui::pos2(full.left(), y), egui::pos2(full.right(), y + 1.0)), 0.0,
                                  egui::Color32::from_black_alpha(a as u8));
                }
                p.rect_filled(band, 0.0, egui::Color32::from_black_alpha(150));
                ui.scope_builder(egui::UiBuilder::new().max_rect(band), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(42.0);
                        ui.label(egui::RichText::new(format!("🔒  {}", self.text)).size(34.0).strong().color(egui::Color32::WHITE));
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(l!("Máy đã bị giáo viên khóa", "Your computer is locked by the teacher")).size(18.0).color(egui::Color32::from_gray(210)));
                    });
                });
                return;
            }
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.33);
                ui.label(egui::RichText::new("🔒").size(80.0).color(egui::Color32::WHITE));
                ui.add_space(18.0);
                ui.label(egui::RichText::new(&self.text).size(34.0).strong().color(egui::Color32::WHITE));
                ui.add_space(10.0);
                ui.label(egui::RichText::new(l!("Máy đã bị giáo viên khóa", "Your computer is locked by the teacher")).size(18.0).color(egui::Color32::from_gray(160)));
            });
        });
        // luôn ở trên cùng và giữ tiêu điểm (không giành liên tục khi đang chạy video)
        let d = std::time::Duration::from_millis(500);
        if self.focused.is_none_or(|t| t.elapsed() >= d) {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.focused = Some(std::time::Instant::now());
        }
        ctx.request_repaint_after(wait.map(|w| w.min(d)).unwrap_or(d));
    }
}

#[cfg(windows)]
fn block_input() {
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{CallNextHookEx, SetWindowsHookExW, WH_KEYBOARD_LL, WH_MOUSE_LL};
    unsafe extern "system" fn swallow(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
        if code >= 0 {
            return 1;
        }
        CallNextHookEx(std::ptr::null_mut(), code, w, l)
    }
    unsafe {
        let m = GetModuleHandleW(std::ptr::null());
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(swallow), m, 0);
        SetWindowsHookExW(WH_MOUSE_LL, Some(swallow), m, 0);
    }
}

#[cfg(target_os = "linux")]
fn block_input() {
    // Giữ bàn phím và chuột cho riêng màn hình khóa (X11). Thử lại tới khi cửa sổ đã hiện.
    std::thread::spawn(|| {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{ConnectionExt, EventMask, GrabMode, GrabStatus};
        let Ok((conn, n)) = x11rb::connect(None) else { return };
        let root = conn.setup().roots[n].root;
        loop {
            let k = conn.grab_keyboard(false, root, x11rb::CURRENT_TIME, GrabMode::ASYNC, GrabMode::ASYNC)
                .ok().and_then(|c| c.reply().ok()).map(|r| r.status == GrabStatus::SUCCESS).unwrap_or(false);
            let p = conn.grab_pointer(false, root, EventMask::NO_EVENT, GrabMode::ASYNC, GrabMode::ASYNC,
                                      x11rb::NONE, x11rb::NONE, x11rb::CURRENT_TIME)
                .ok().and_then(|c| c.reply().ok()).map(|r| r.status == GrabStatus::SUCCESS).unwrap_or(false);
            if k && p {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        // giữ kết nối (và quyền giữ phím) tới khi tiến trình bị dừng
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    });
}

#[cfg(not(any(windows, target_os = "linux")))]
fn block_input() {}

// ------------------------------------------------------------------ tin nhắn
struct Message {
    text: String,
}

impl eframe::App for Message {
    fn ui(&mut self, root: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::WHITE).inner_margin(24)).show(root, |ui| {
            ui.label(egui::RichText::new(l!("Tin nhắn từ giáo viên", "Message from the teacher")).size(15.0).strong().color(INDIGO));
            ui.add_space(10.0);
            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                ui.label(egui::RichText::new(&self.text).size(20.0).color(egui::Color32::from_rgb(15, 23, 42)));
            });
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
                let b = egui::Button::new(egui::RichText::new(l!("Đã hiểu", "OK")).size(16.0).color(egui::Color32::WHITE)).fill(INDIGO);
                if ui.add_sized([120.0, 36.0], b).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

// ------------------------------------------------------------------ xem trình chiếu
fn read_tiles(shared: Arc<Mutex<Option<egui::ColorImage>>>, ctx: egui::Context) {
    let mut stdin = std::io::stdin().lock();
    let mut canvas = lc_core::img::TileCanvas::default();
    loop {
        let mut len = [0u8; 4];
        if stdin.read_exact(&mut len).is_err() {
            std::process::exit(0); // Agent đã dừng trình chiếu
        }
        let n = u32::from_be_bytes(len) as usize;
        if n > lc_core::proto::MAX_FRAME {
            std::process::exit(1);
        }
        let mut pkt = vec![0u8; n];
        if stdin.read_exact(&mut pkt).is_err() {
            std::process::exit(0);
        }
        if canvas.apply(&pkt).is_ok() && canvas.width > 0 {
            let img = egui::ColorImage::from_rgba_unmultiplied([canvas.width as usize, canvas.height as usize], &canvas.rgba);
            *shared.lock().unwrap() = Some(img);
            ctx.request_repaint();
        }
    }
}

struct View {
    shared: Arc<Mutex<Option<egui::ColorImage>>>,
    tex: Option<egui::TextureHandle>,
    fullscreen: bool,
}

impl eframe::App for View {
    fn ui(&mut self, root: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        if self.fullscreen {
            keep_open(ctx);
            // luôn ở trên cùng, giữ tiêu điểm, bị thu nhỏ thì phóng lại
            let (min, full, focus) = ctx.input(|i| (i.viewport().minimized, i.viewport().fullscreen, i.viewport().focused));
            if min == Some(true) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            }
            if full == Some(false) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
            if focus != Some(true) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        if let Some(img) = self.shared.lock().unwrap().take() {
            match &mut self.tex {
                Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                None => self.tex = Some(ctx.load_texture("screen", img, egui::TextureOptions::LINEAR)),
            }
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::BLACK)).show(root, |ui| {
            let avail = ui.available_rect_before_wrap();
            match &self.tex {
                Some(t) => {
                    let [w, h] = t.size();
                    let scale = (avail.width() / w as f32).min(avail.height() / h as f32);
                    let size = egui::vec2(w as f32 * scale, h as f32 * scale);
                    let rect = egui::Rect::from_center_size(avail.center(), size);
                    ui.painter().image(t.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                       egui::Color32::WHITE);
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new(l!("Đang nhận hình từ giáo viên...", "Receiving the teacher's screen...")).size(22.0).color(egui::Color32::GRAY));
                    });
                }
            }
        });
    }
}

// ------------------------------------------------------------------ chỉ báo đang được xem
struct Badge;

impl eframe::App for Badge {
    fn ui(&mut self, root: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        keep_open(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(220, 38, 38)).inner_margin(egui::Margin::symmetric(12, 8)))
            .show(root, |ui| {
                ui.label(egui::RichText::new(l!("● Giáo viên đang xem màn hình của bạn", "● The teacher is viewing your screen")).size(15.0).strong().color(egui::Color32::WHITE));
            });
    }
}

// ------------------------------------------------------------------ dải "đang thi"
struct Exam;

impl eframe::App for Exam {
    fn ui(&mut self, root: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        keep_open(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(217, 119, 6)).inner_margin(egui::Margin::symmetric(12, 7)))
            .show(root, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(l!("ĐANG TRONG GIỜ KIỂM TRA", "EXAM IN PROGRESS")).size(15.0).strong().color(egui::Color32::WHITE));
                });
            });
    }
}

// ------------------------------------------------------------------ trắc nghiệm
struct QuizWin {
    quiz: lc_core::proto::Quiz,
    answers: Vec<i32>,
    deadline: Option<std::time::Instant>,
    confirm: bool,
    sent_done: bool,
}

impl QuizWin {
    fn emit(&self, done: bool) {
        use std::io::Write;
        let line = serde_json::json!({"answers": self.answers, "done": done});
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

impl eframe::App for QuizWin {
    fn ui(&mut self, root: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = &root.ctx().clone();
        keep_open(ctx);
        let left = self.deadline.map(|d| d.saturating_duration_since(std::time::Instant::now()).as_secs());
        if left == Some(0) && !self.sent_done {
            self.sent_done = true;
            self.emit(true);
        }
        let answered = self.answers.iter().filter(|a| **a >= 0).count();
        egui::Panel::top("head").frame(egui::Frame::new().fill(INDIGO).inner_margin(egui::Margin::symmetric(18, 12))).show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&self.quiz.title).size(20.0).strong().color(egui::Color32::WHITE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(l) = left {
                        let c = if l < 60 { egui::Color32::from_rgb(254, 202, 202) } else { egui::Color32::WHITE };
                        ui.label(egui::RichText::new(format!("⏱ {:02}:{:02}", l / 60, l % 60)).size(20.0).strong().color(c));
                    }
                    ui.label(egui::RichText::new(l!("Đã làm {answered}/{}", "Answered {answered}/{}", self.quiz.questions.len())).size(15.0)
                        .color(egui::Color32::from_rgb(204, 251, 241)));
                });
            });
        });
        egui::Panel::bottom("foot").frame(egui::Frame::new().fill(egui::Color32::from_rgb(241, 245, 249)).inner_margin(egui::Margin::symmetric(18, 10))).show(root, |ui| {
            ui.horizontal(|ui| {
                if self.sent_done {
                    ui.label(egui::RichText::new(l!("Đã nộp bài. Chờ giáo viên...", "Handed in. Waiting for the teacher...")).size(16.0).strong().color(INDIGO));
                } else if self.confirm {
                    ui.label(egui::RichText::new(l!("Nộp bài? Còn {} câu chưa làm.", "Hand in? {} questions unanswered.", self.quiz.questions.len() - answered)).size(15.0).color(INK));
                    if ui.add_sized([90.0, 34.0], egui::Button::new(egui::RichText::new(l!("Nộp", "Hand in")).size(16.0).color(egui::Color32::WHITE)).fill(INDIGO)).clicked() {
                        self.sent_done = true;
                        self.emit(true);
                    }
                    if ui.add_sized([100.0, 34.0], egui::Button::new(egui::RichText::new(l!("Làm tiếp", "Keep working")).size(16.0).color(INK))).clicked() {
                        self.confirm = false;
                    }
                } else {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let b = egui::Button::new(egui::RichText::new(l!("Nộp bài", "Hand in")).size(16.0).color(egui::Color32::WHITE)).fill(INDIGO);
                        if ui.add_sized([130.0, 36.0], b).clicked() {
                            self.confirm = true;
                        }
                    });
                }
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::WHITE).inner_margin(18)).show(root, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let mut changed = false;
                for (i, q) in self.quiz.questions.iter().enumerate() {
                    ui.label(egui::RichText::new(l!("Câu {}. {}", "Question {}. {}", i + 1, q.q)).size(17.0).strong().color(INK));
                    ui.add_space(4.0);
                    for (j, opt) in q.options.iter().enumerate() {
                        let letter = (b'A' + j as u8) as char;
                        let sel = self.answers[i] == j as i32;
                        let r = ui.add_enabled(!self.sent_done, egui::RadioButton::new(sel, egui::RichText::new(format!("{letter}. {opt}")).size(16.0).color(INK)));
                        if r.clicked() && !sel {
                            self.answers[i] = j as i32;
                            changed = true;
                        }
                    }
                    ui.add_space(14.0);
                }
                if changed {
                    self.emit(false);
                }
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
