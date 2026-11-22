//! Chụp màn hình (kèm con trỏ chuột).
//! Windows: GDI BitBlt màn hình chính. Linux: X11 GetImage toàn bộ màn hình (Wayland chưa hỗ trợ).
//! Ảnh trả về dạng BGRA, 4 byte mỗi điểm ảnh, không có khoảng đệm giữa các dòng.

use anyhow::Result;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

/// Báo cho Windows biết chương trình hiểu DPI, để tọa độ chụp và tọa độ chuột đều là điểm ảnh thật.
pub fn init_dpi() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Frame;
    use anyhow::{anyhow, Result};
    use x11rb::connection::Connection;
    use x11rb::protocol::xfixes::ConnectionExt as _;
    use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};
    use x11rb::rust_connection::RustConnection;

    pub struct Capturer {
        conn: RustConnection,
        root: u32,
        xfixes: bool,
    }

    impl Capturer {
        pub fn new() -> Result<Self> {
            let (conn, n) = x11rb::connect(None).map_err(|e| anyhow!("Không kết nối được X11 (Wayland chưa được hỗ trợ): {e}"))?;
            let root = conn.setup().roots[n].root;
            let xfixes = conn.xfixes_query_version(5, 0).ok().and_then(|c| c.reply().ok()).is_some();
            Ok(Capturer { conn, root, xfixes })
        }

        pub fn size(&self) -> (u32, u32) {
            let g = self.conn.get_geometry(self.root).ok().and_then(|c| c.reply().ok());
            g.map(|g| (g.width as u32, g.height as u32)).unwrap_or((0, 0))
        }

        pub fn capture(&mut self) -> Result<Frame> {
            let (w, h) = self.size();
            let img = self.conn.get_image(ImageFormat::Z_PIXMAP, self.root, 0, 0, w as u16, h as u16, !0)?.reply()?;
            let mut bgra = img.data;
            if bgra.len() != (w * h * 4) as usize {
                return Err(anyhow!("Chỉ hỗ trợ màn hình 24/32 bit"));
            }
            for px in bgra.chunks_exact_mut(4) {
                px[3] = 255;
            }
            let mut f = Frame { width: w, height: h, bgra };
            if self.xfixes {
                if let Ok(Ok(c)) = self.conn.xfixes_get_cursor_image().map(|c| c.reply()) {
                    let pixels: Vec<u32> = c.cursor_image;
                    super::blend_cursor(&mut f, c.x as i32 - c.xhot as i32, c.y as i32 - c.yhot as i32,
                                        c.width as u32, c.height as u32, &pixels);
                }
            }
            Ok(f)
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::Frame;
    use anyhow::{anyhow, Result};
    use std::mem::{size_of, zeroed};
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    pub struct Capturer;

    impl Capturer {
        pub fn new() -> Result<Self> {
            Ok(Capturer)
        }

        pub fn size(&self) -> (u32, u32) {
            unsafe { (GetSystemMetrics(SM_CXSCREEN) as u32, GetSystemMetrics(SM_CYSCREEN) as u32) }
        }

        pub fn capture(&mut self) -> Result<Frame> {
            let (w, h) = self.size();
            if w == 0 || h == 0 {
                return Err(anyhow!("Không đọc được kích thước màn hình"));
            }
            unsafe {
                let screen = GetDC(null_mut());
                let mem = CreateCompatibleDC(screen);
                let bmp = CreateCompatibleBitmap(screen, w as i32, h as i32);
                let old = SelectObject(mem, bmp);
                let ok = BitBlt(mem, 0, 0, w as i32, h as i32, screen, 0, 0, SRCCOPY | CAPTUREBLT);

                let mut ci: CURSORINFO = zeroed();
                ci.cbSize = size_of::<CURSORINFO>() as u32;
                if GetCursorInfo(&mut ci) != 0 && ci.flags == CURSOR_SHOWING {
                    let mut ii: ICONINFO = zeroed();
                    if GetIconInfo(ci.hCursor, &mut ii) != 0 {
                        DrawIconEx(mem, ci.ptScreenPos.x - ii.xHotspot as i32, ci.ptScreenPos.y - ii.yHotspot as i32,
                                   ci.hCursor, 0, 0, 0, null_mut(), DI_NORMAL);
                        if !ii.hbmMask.is_null() {
                            DeleteObject(ii.hbmMask);
                        }
                        if !ii.hbmColor.is_null() {
                            DeleteObject(ii.hbmColor);
                        }
                    }
                }

                let mut bi: BITMAPINFO = zeroed();
                bi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                bi.bmiHeader.biWidth = w as i32;
                bi.bmiHeader.biHeight = -(h as i32); // từ trên xuống
                bi.bmiHeader.biPlanes = 1;
                bi.bmiHeader.biBitCount = 32;
                bi.bmiHeader.biCompression = BI_RGB;
                let mut buf = vec![0u8; (w * h * 4) as usize];
                let lines = GetDIBits(mem, bmp, 0, h, buf.as_mut_ptr() as *mut _, &mut bi, DIB_RGB_COLORS);

                SelectObject(mem, old);
                DeleteObject(bmp);
                DeleteDC(mem);
                ReleaseDC(null_mut(), screen);
                if ok == 0 || lines == 0 {
                    // Màn hình đang khóa / UAC: trả về khung đen thay vì lỗi
                    return Ok(Frame { width: w, height: h, bgra: vec![0; (w * h * 4) as usize] });
                }
                for px in buf.chunks_exact_mut(4) {
                    px[3] = 255;
                }
                Ok(Frame { width: w, height: h, bgra: buf })
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use super::Frame;
    use anyhow::{bail, Result};
    pub struct Capturer;
    impl Capturer {
        pub fn new() -> Result<Self> { bail!("Hệ điều hành chưa được hỗ trợ") }
        pub fn size(&self) -> (u32, u32) { (0, 0) }
        pub fn capture(&mut self) -> Result<Frame> { bail!("Hệ điều hành chưa được hỗ trợ") }
    }
}

pub use imp::Capturer;

/// Vẽ con trỏ ARGB (đã nhân alpha) lên khung hình BGRA.
#[allow(dead_code)]
fn blend_cursor(f: &mut Frame, x0: i32, y0: i32, cw: u32, ch: u32, argb: &[u32]) {
    for cy in 0..ch as i32 {
        let y = y0 + cy;
        if y < 0 || y >= f.height as i32 {
            continue;
        }
        for cx in 0..cw as i32 {
            let x = x0 + cx;
            if x < 0 || x >= f.width as i32 {
                continue;
            }
            let p = argb[(cy as u32 * cw + cx as u32) as usize];
            let a = (p >> 24) & 0xff;
            if a == 0 {
                continue;
            }
            let i = ((y as u32 * f.width + x as u32) * 4) as usize;
            let inv = 255 - a;
            for (k, sh) in [(0usize, 0u32), (1, 8), (2, 16)] {
                let src = (p >> sh) & 0xff; // đã nhân alpha
                let dst = f.bgra[i + k] as u32;
                f.bgra[i + k] = (src + dst * inv / 255).min(255) as u8;
            }
        }
    }
}

/// Chụp một lần (tiện cho ảnh thu nhỏ).
pub fn capture_once() -> Result<Frame> {
    Capturer::new()?.capture()
}
