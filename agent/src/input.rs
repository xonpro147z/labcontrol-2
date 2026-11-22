//! Điều khiển chuột/bàn phím từ xa. Chạy trên một luồng riêng vì Enigo không dùng chung giữa các luồng.

use std::sync::mpsc;

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use lc_core::proto::InputEv;

pub struct Input {
    tx: mpsc::Sender<InputEv>,
}

impl Input {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<InputEv>();
        std::thread::spawn(move || {
            let mut enigo = match Enigo::new(&Settings::default()) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Không điều khiển được chuột/bàn phím: {e}");
                    return;
                }
            };
            let mut cap = lc_core::capture::Capturer::new().ok();
            while let Ok(ev) = rx.recv() {
                if let Err(e) = apply(&mut enigo, &mut cap, ev) {
                    tracing::debug!("Lỗi điều khiển: {e}");
                }
            }
        });
        Input { tx }
    }

    pub fn send(&self, ev: InputEv) {
        let _ = self.tx.send(ev);
    }
}

fn apply(e: &mut Enigo, cap: &mut Option<lc_core::capture::Capturer>, ev: InputEv) -> anyhow::Result<()> {
    match ev {
        InputEv::MouseMove { x, y } => {
            let (w, h) = cap.as_ref().map(|c| c.size()).unwrap_or((1920, 1080));
            let px = (x.clamp(0.0, 1.0) * (w.saturating_sub(1)) as f32).round() as i32;
            let py = (y.clamp(0.0, 1.0) * (h.saturating_sub(1)) as f32).round() as i32;
            e.move_mouse(px, py, Coordinate::Abs)?;
        }
        InputEv::MouseButton { button, down } => {
            let b = match button.as_str() {
                "right" => Button::Right,
                "middle" => Button::Middle,
                _ => Button::Left,
            };
            e.button(b, if down { Direction::Press } else { Direction::Release })?;
        }
        InputEv::Wheel { dx, dy } => {
            if dy != 0 {
                e.scroll(dy.signum() * (dy.abs() / 100).max(1), Axis::Vertical)?;
            }
            if dx != 0 {
                e.scroll(dx.signum() * (dx.abs() / 100).max(1), Axis::Horizontal)?;
            }
        }
        InputEv::Key { key, down, .. } => {
            if let Some(k) = map_key(&key) {
                e.key(k, if down { Direction::Press } else { Direction::Release })?;
            }
        }
        InputEv::Text { text } => {
            if !text.is_empty() {
                e.text(&text)?;
            }
        }
    }
    Ok(())
}

/// Đổi tên phím theo chuẩn trình duyệt (KeyboardEvent.key) sang phím của Enigo.
pub fn map_key(k: &str) -> Option<Key> {
    Some(match k {
        "Enter" => Key::Return,
        "Backspace" => Key::Backspace,
        "Tab" => Key::Tab,
        "Escape" => Key::Escape,
        " " => Key::Space,
        "Delete" => Key::Delete,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "ArrowLeft" => Key::LeftArrow,
        "ArrowRight" => Key::RightArrow,
        "ArrowUp" => Key::UpArrow,
        "ArrowDown" => Key::DownArrow,
        "Shift" => Key::Shift,
        "Control" => Key::Control,
        "Alt" => Key::Alt,
        "Meta" => Key::Meta,
        "CapsLock" => Key::CapsLock,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        s => {
            let mut it = s.chars();
            let c = it.next()?;
            if it.next().is_some() {
                return None; // tên phím lạ (AudioVolumeUp...) thì bỏ qua
            }
            Key::Unicode(c)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys() {
        assert!(matches!(map_key("Enter"), Some(Key::Return)));
        assert!(matches!(map_key("ệ"), Some(Key::Unicode('ệ'))));
        assert!(map_key("AudioVolumeUp").is_none());
    }
}
