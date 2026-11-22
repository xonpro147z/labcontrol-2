//! Thu nhỏ ảnh, mã hóa/giải mã JPEG, và mã hóa "ô thay đổi" cho trình chiếu.
//!
//! Trình chiếu không gửi lại cả màn hình mỗi khung hình: màn hình chia thành ô 64×64,
//! chỉ những ô thay đổi so với khung trước mới được nén JPEG và gửi đi. Khi giáo viên đang
//! giảng (màn hình ít thay đổi), mỗi khung chỉ vài KB nên rất mượt cả khi cả phòng cùng xem.

use anyhow::{anyhow, Result};
use jpeg_encoder::{ColorType, Encoder};

use crate::capture::Frame;

/// Thu nhỏ (lọc trung bình theo vùng) về chiều rộng tối đa `max_w`. Giữ BGRA.
pub fn downscale(f: &Frame, max_w: u32) -> Frame {
    if f.width <= max_w || max_w == 0 {
        return Frame { width: f.width, height: f.height, bgra: f.bgra.clone() };
    }
    let nw = max_w;
    let nh = ((f.height as u64 * nw as u64) / f.width as u64).max(1) as u32;
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for oy in 0..nh {
        let y0 = oy * f.height / nh;
        let y1 = ((oy + 1) * f.height / nh).max(y0 + 1).min(f.height);
        for ox in 0..nw {
            let x0 = ox * f.width / nw;
            let x1 = ((ox + 1) * f.width / nw).max(x0 + 1).min(f.width);
            let (mut b, mut g, mut r, mut n) = (0u32, 0u32, 0u32, 0u32);
            // lấy mẫu thưa (tối đa 3×3 điểm) để nhanh trên máy yếu
            let sy = ((y1 - y0) / 3).max(1);
            let sx = ((x1 - x0) / 3).max(1);
            let mut y = y0;
            while y < y1 {
                let mut x = x0;
                while x < x1 {
                    let i = ((y * f.width + x) * 4) as usize;
                    b += f.bgra[i] as u32;
                    g += f.bgra[i + 1] as u32;
                    r += f.bgra[i + 2] as u32;
                    n += 1;
                    x += sx;
                }
                y += sy;
            }
            let o = ((oy * nw + ox) * 4) as usize;
            out[o] = (b / n) as u8;
            out[o + 1] = (g / n) as u8;
            out[o + 2] = (r / n) as u8;
            out[o + 3] = 255;
        }
    }
    Frame { width: nw, height: nh, bgra: out }
}

pub fn jpeg(f: &Frame, quality: u8) -> Result<Vec<u8>> {
    jpeg_raw(f.width, f.height, &f.bgra, quality)
}

pub fn jpeg_raw(w: u32, h: u32, bgra: &[u8], quality: u8) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity((w * h / 4) as usize);
    Encoder::new(&mut out, quality.clamp(10, 95))
        .encode(bgra, w as u16, h as u16, ColorType::Bgra)
        .map_err(|e| anyhow!("Lỗi nén JPEG: {e}"))?;
    Ok(out)
}

/// Giải mã JPEG ra RGBA.
pub fn decode_rgba(data: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    use zune_core::bytestream::ZCursor;
    use zune_core::colorspace::ColorSpace;
    use zune_core::options::DecoderOptions;
    let opts = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut d = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(data), opts);
    let px = d.decode().map_err(|e| anyhow!("Lỗi giải mã JPEG: {e:?}"))?;
    let (w, h) = d.dimensions().ok_or_else(|| anyhow!("JPEG thiếu kích thước"))?;
    Ok((w as u32, h as u32, px))
}

// ------------------------------------------------------------------ ô thay đổi (trình chiếu)
pub const TILE: u32 = 64;

pub struct TileEncoder {
    prev: Vec<u8>,
    w: u32,
    h: u32,
}

impl Default for TileEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl TileEncoder {
    pub fn new() -> Self {
        TileEncoder { prev: Vec::new(), w: 0, h: 0 }
    }

    fn tile_changed(&self, f: &Frame, tx: u32, ty: u32) -> bool {
        let x0 = tx * TILE;
        let x1 = (x0 + TILE).min(f.width);
        let y0 = ty * TILE;
        let y1 = (y0 + TILE).min(f.height);
        let row = ((x1 - x0) * 4) as usize;
        for y in y0..y1 {
            let i = ((y * f.width + x0) * 4) as usize;
            if f.bgra[i..i + row] != self.prev[i..i + row] {
                return true;
            }
        }
        false
    }

    /// Trả về gói các vùng thay đổi, hoặc None nếu màn hình không đổi.
    /// Định dạng: u16 W, u16 H, u16 n, rồi n lần [u16 x, u16 y, u16 w, u16 h, u32 len, JPEG].
    pub fn encode(&mut self, f: &Frame, force_full: bool, quality: u8) -> Result<Option<Vec<u8>>> {
        let full = force_full || f.width != self.w || f.height != self.h || self.prev.len() != f.bgra.len();
        let tw = f.width.div_ceil(TILE);
        let th = f.height.div_ceil(TILE);
        let mut rects: Vec<(u32, u32, u32, u32)> = Vec::new();
        for ty in 0..th {
            let mut run: Option<u32> = None;
            for tx in 0..=tw {
                let changed = tx < tw && (full || self.tile_changed(f, tx, ty));
                match (changed, run) {
                    (true, None) => run = Some(tx),
                    (false, Some(s)) => {
                        let x0 = s * TILE;
                        let x1 = (tx * TILE).min(f.width);
                        let y0 = ty * TILE;
                        let y1 = (y0 + TILE).min(f.height);
                        rects.push((x0, y0, x1 - x0, y1 - y0));
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        if rects.is_empty() {
            return Ok(None);
        }
        let mut out = Vec::new();
        out.extend_from_slice(&(f.width as u16).to_be_bytes());
        out.extend_from_slice(&(f.height as u16).to_be_bytes());
        out.extend_from_slice(&(rects.len() as u16).to_be_bytes());
        let mut sub = Vec::new();
        for (x, y, w, h) in rects {
            sub.clear();
            for yy in y..y + h {
                let i = ((yy * f.width + x) * 4) as usize;
                sub.extend_from_slice(&f.bgra[i..i + (w * 4) as usize]);
            }
            let j = jpeg_raw(w, h, &sub, quality)?;
            for v in [x as u16, y as u16, w as u16, h as u16] {
                out.extend_from_slice(&v.to_be_bytes());
            }
            out.extend_from_slice(&(j.len() as u32).to_be_bytes());
            out.extend_from_slice(&j);
        }
        self.prev.clear();
        self.prev.extend_from_slice(&f.bgra);
        self.w = f.width;
        self.h = f.height;
        Ok(Some(out))
    }
}

/// Ghép các gói ô vào một khung RGBA.
#[derive(Default)]
pub struct TileCanvas {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl TileCanvas {
    pub fn apply(&mut self, pkt: &[u8]) -> Result<()> {
        let rd16 = |b: &[u8], i: usize| -> Result<u32> {
            b.get(i..i + 2).map(|s| u16::from_be_bytes([s[0], s[1]]) as u32).ok_or_else(|| anyhow!("gói ô bị cắt"))
        };
        let w = rd16(pkt, 0)?;
        let h = rd16(pkt, 2)?;
        let n = rd16(pkt, 4)?;
        if w != self.width || h != self.height {
            self.width = w;
            self.height = h;
            self.rgba = vec![0; (w * h * 4) as usize];
        }
        let mut p = 6usize;
        for _ in 0..n {
            let (x, y, tw, th) = (rd16(pkt, p)?, rd16(pkt, p + 2)?, rd16(pkt, p + 4)?, rd16(pkt, p + 6)?);
            let len = pkt.get(p + 8..p + 12).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize)
                .ok_or_else(|| anyhow!("gói ô bị cắt"))?;
            p += 12;
            let j = pkt.get(p..p + len).ok_or_else(|| anyhow!("gói ô bị cắt"))?;
            p += len;
            let (dw, dh, px) = decode_rgba(j)?;
            let cw = dw.min(tw).min(w.saturating_sub(x));
            let ch = dh.min(th).min(h.saturating_sub(y));
            for yy in 0..ch {
                let src = (yy * dw * 4) as usize;
                let dst = (((y + yy) * w + x) * 4) as usize;
                self.rgba[dst..dst + (cw * 4) as usize].copy_from_slice(&px[src..src + (cw * 4) as usize]);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32, v: u8) -> Frame {
        Frame { width: w, height: h, bgra: vec![v; (w * h * 4) as usize] }
    }

    #[test]
    fn tiles_roundtrip() {
        let mut enc = TileEncoder::new();
        let mut f = frame(300, 200, 40);
        let full = enc.encode(&f, false, 80).unwrap().unwrap();
        assert!(enc.encode(&f, false, 80).unwrap().is_none(), "không đổi thì không gửi");
        // đổi một ô
        for y in 70..90 {
            for x in 130..150 {
                let i = ((y * 300 + x) * 4) as usize;
                f.bgra[i..i + 3].copy_from_slice(&[0, 0, 255]);
            }
        }
        let delta = enc.encode(&f, false, 80).unwrap().unwrap();
        assert!(delta.len() < full.len());
        let mut c = TileCanvas::default();
        c.apply(&full).unwrap();
        c.apply(&delta).unwrap();
        let i = ((80 * 300 + 140) * 4) as usize;
        assert!(c.rgba[i] > 200 && c.rgba[i + 2] < 60, "ô đổi phải thành màu đỏ: {:?}", &c.rgba[i..i + 4]);
        let j = ((10 * 300 + 10) * 4) as usize;
        assert!((c.rgba[j] as i32 - 40).abs() < 6);
    }

    #[test]
    fn scale_and_jpeg() {
        let f = frame(1920, 1080, 100);
        let s = downscale(&f, 320);
        assert_eq!((s.width, s.height), (320, 180));
        let j = jpeg(&s, 60).unwrap();
        let (w, h, px) = decode_rgba(&j).unwrap();
        assert_eq!((w, h, px.len()), (320, 180, 320 * 180 * 4));
    }
}
