//! Giao thức giữa Console và Agent.
//!
//! Mỗi gói: 4 byte độ dài (big-endian, tính từ byte loại) + 1 byte loại + dữ liệu.
//! Loại 0 là JSON điều khiển; các loại khác là dữ liệu nhị phân (ảnh, ô trình chiếu, file).

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const K_CTRL: u8 = 0;
pub const K_THUMB: u8 = 1; // ảnh thu nhỏ (JPEG)
pub const K_STREAM: u8 = 2; // khung hình xem lớn / điều khiển (JPEG)
pub const K_TILES: u8 = 3; // trình chiếu: các ô thay đổi
pub const K_FILE: u8 = 4; // một đoạn file: 8 byte id + dữ liệu

pub const MAX_FRAME: usize = 32 << 20;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub os: String,
    pub user: String,
    pub version: String,
    pub room: String,
    pub screen_w: u32,
    pub screen_h: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Status {
    pub user: String,
    pub app: String,
    pub title: String,
    pub locked: bool,
    pub showing: bool,
    pub viewed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum InputEv {
    /// Tọa độ chuẩn hóa 0..1 trên màn hình chính của máy học sinh
    MouseMove { x: f32, y: f32 },
    MouseButton { button: String, down: bool },
    Wheel { dx: i32, dy: i32 },
    /// `key` theo chuẩn KeyboardEvent.key của trình duyệt ("a", "Enter", "ArrowLeft", ...)
    Key { key: String, code: String, down: bool },
    /// Gõ cả chuỗi (dán chữ, tiếng Việt có dấu)
    Text { text: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Ctrl {
    // ---------------- Agent -> Console
    Hello { nonce: String, proto: u32, info: AgentInfo },
    AuthOk { info: AgentInfo },
    AuthFail { reason: String },
    Status { status: Status },
    FileDone { id: u64, ok: bool, path: String, error: String },
    Notice { text: String },
    // ---------------- Console -> Agent
    Auth { teacher: String, teacher_name: String, sig: String },
    GetThumb { width: u32, quality: u8 },
    GetStatus,
    StreamStart { width: u32, fps: u32, quality: u8 },
    StreamStop,
    Input { ev: InputEv },
    Lock { message: String },
    Unlock,
    Message { text: String },
    OpenUrl { url: String },
    Power { action: String },
    FileBegin { id: u64, name: String, size: u64 },
    FileEnd { id: u64 },
    ShowStart { fullscreen: bool },
    ShowStop,
    Ping,
}

pub async fn write_frame<W: AsyncWrite + Unpin>(w: &mut W, kind: u8, data: &[u8]) -> Result<()> {
    let len = (data.len() + 1) as u32;
    let mut head = [0u8; 5];
    head[..4].copy_from_slice(&len.to_be_bytes());
    head[4] = kind;
    w.write_all(&head).await?;
    w.write_all(data).await?;
    Ok(())
}

pub async fn write_ctrl<W: AsyncWrite + Unpin>(w: &mut W, c: &Ctrl) -> Result<()> {
    write_frame(w, K_CTRL, &serde_json::to_vec(c)?).await
}

pub fn ctrl_bytes(c: &Ctrl) -> Vec<u8> {
    serde_json::to_vec(c).unwrap_or_default()
}

/// Đọc một gói. Trả về (loại, dữ liệu).
pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> Result<(u8, Vec<u8>)> {
    let mut head = [0u8; 5];
    r.read_exact(&mut head).await?;
    let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
    if len == 0 || len > MAX_FRAME {
        bail!("gói tin không hợp lệ ({len} byte)");
    }
    let mut buf = vec![0u8; len - 1];
    r.read_exact(&mut buf).await?;
    Ok((head[4], buf))
}

pub fn parse_ctrl(data: &[u8]) -> Result<Ctrl> {
    Ok(serde_json::from_slice(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(flavor = "current_thread")]
    async fn roundtrip() {
        let mut buf = Vec::new();
        write_ctrl(&mut buf, &Ctrl::Lock { message: "Chú ý".into() }).await.unwrap();
        write_frame(&mut buf, K_THUMB, &[1, 2, 3]).await.unwrap();
        let mut r = &buf[..];
        let (k, d) = read_frame(&mut r).await.unwrap();
        assert_eq!(k, K_CTRL);
        assert!(matches!(parse_ctrl(&d).unwrap(), Ctrl::Lock { message } if message == "Chú ý"));
        let (k, d) = read_frame(&mut r).await.unwrap();
        assert_eq!((k, d), (K_THUMB, vec![1, 2, 3]));
    }
}
