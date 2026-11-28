//! Ảnh/video nền cho màn hình khóa. Gói "LCM1": fps (u16, 0 = ảnh tĩnh), số khung (u32),
//! rồi từng khung JPEG (u32 độ dài + dữ liệu). Máy giáo viên đóng gói, máy học sinh giữ trong bộ nhớ đệm.

use anyhow::{bail, Result};

pub const MAGIC: &[u8; 4] = b"LCM1";
/// Giới hạn kích thước gói (byte)
pub const MAX_SIZE: u64 = 48 << 20;
pub const MAX_FRAMES: usize = 600;

pub fn pack(fps: u16, frames: &[Vec<u8>]) -> Vec<u8> {
    let total: usize = frames.iter().map(|f| f.len() + 4).sum();
    let mut out = Vec::with_capacity(10 + total);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&fps.to_be_bytes());
    out.extend_from_slice(&(frames.len() as u32).to_be_bytes());
    for f in frames {
        out.extend_from_slice(&(f.len() as u32).to_be_bytes());
        out.extend_from_slice(f);
    }
    out
}

/// Trả về (fps, các khung JPEG)
pub fn unpack(data: &[u8]) -> Result<(u16, Vec<Vec<u8>>)> {
    if data.len() < 10 || &data[..4] != MAGIC {
        bail!("không phải gói màn hình khóa");
    }
    let fps = u16::from_be_bytes([data[4], data[5]]);
    let n = u32::from_be_bytes(data[6..10].try_into().unwrap()) as usize;
    if n == 0 || n > MAX_FRAMES {
        bail!("số khung không hợp lệ ({n})");
    }
    let mut frames = Vec::with_capacity(n);
    let mut p = 10;
    for _ in 0..n {
        if p + 4 > data.len() {
            bail!("gói bị cắt");
        }
        let len = u32::from_be_bytes(data[p..p + 4].try_into().unwrap()) as usize;
        p += 4;
        if p + len > data.len() {
            bail!("gói bị cắt");
        }
        frames.push(data[p..p + len].to_vec());
        p += len;
    }
    Ok((fps, frames))
}

#[cfg(test)]
mod tests {
    #[test]
    fn roundtrip() {
        let f = vec![vec![1u8, 2, 3], vec![9u8; 100]];
        let p = super::pack(12, &f);
        let (fps, g) = super::unpack(&p).unwrap();
        assert_eq!(fps, 12);
        assert_eq!(g, f);
        assert!(super::unpack(&p[..p.len() - 1]).is_err());
    }
}
