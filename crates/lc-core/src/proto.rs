//! Giao thức giữa Console (máy giáo viên) và Agent (máy học sinh), phiên bản 3.
//!
//! Từ bản 3, **máy học sinh tự tìm và tự nối về máy giáo viên** (giống iClassroom/NetSupport):
//! chỉ máy giáo viên phải mở tường lửa. Máy giáo viên là máy chủ TLS; Agent là máy khách.
//!
//! Mỗi gói: 4 byte độ dài (big-endian, tính từ byte loại) + 1 byte loại + dữ liệu.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const K_CTRL: u8 = 0;
pub const K_THUMB: u8 = 1; // ảnh thu nhỏ (JPEG)
pub const K_STREAM: u8 = 2; // khung hình xem lớn / điều khiển (JPEG)
pub const K_TILES: u8 = 3; // trình chiếu: các ô thay đổi
pub const K_FILE: u8 = 4; // một đoạn file: 8 byte id + dữ liệu (cả hai chiều)

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
    /// "windows" | "linux" (từ bản 3.0.2)
    #[serde(default)]
    pub platform: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Status {
    pub user: String,
    pub app: String,
    pub title: String,
    pub locked: bool,
    pub showing: bool,
    pub viewed: bool,
    #[serde(default)]
    pub exam: bool,
    #[serde(default)]
    pub web_blocked: bool,
    #[serde(default)]
    pub tools_blocked: bool,
    /// Có Helper (dịch vụ quyền cao) để chặn web không
    #[serde(default)]
    pub helper: bool,
}

/// Chính sách giới hạn trên máy học sinh.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Policy {
    /// Tên miền bị chặn (vd "youtube.com"). Cần Helper quyền cao.
    #[serde(default)]
    pub block_web: Vec<String>,
    /// Chặn cmd, PowerShell, Registry, Task Manager, Terminal...
    #[serde(default)]
    pub block_tools: bool,
    /// Tên tiến trình bị đóng ngay khi mở (vd "chrome.exe", "zalo.exe")
    #[serde(default)]
    pub block_apps: Vec<String>,
    /// Chế độ thi: chặn công cụ + đóng trình duyệt, ứng dụng nhắn tin; hiện dải "Đang thi"
    #[serde(default)]
    pub exam: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuizQuestion {
    pub q: String,
    pub options: Vec<String>,
    /// Chỉ máy giáo viên biết đáp án; không gửi xuống máy học sinh
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Quiz {
    pub id: String,
    pub title: String,
    pub minutes: u32,
    pub questions: Vec<QuizQuestion>,
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
    Text { text: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Ctrl {
    // ================= bắt tay =================
    /// Máy giáo viên → Agent, ngay khi Agent nối vào
    TeacherHello { proto: u32, nonce: String, teacher: String, teacher_name: String, room: String },
    /// Agent → máy giáo viên. mode: "key" (đã ghép), "pair" (ghép bằng mật khẩu phòng), "none" (chưa có mật khẩu)
    AgentHello { info: AgentInfo, nonce: String, mode: String },
    /// Máy giáo viên chứng minh mình giữ khóa riêng (đã ghép trước đó)
    Auth { sig: String },
    /// Ghép bằng mật khẩu phòng (SPAKE2): Agent gửi trước
    PairStart { msg: String },
    /// Máy giáo viên trả lời + xác nhận đã biết mật khẩu
    PairReply { msg: String, confirm: String },
    /// Agent xác nhận đã biết mật khẩu
    PairConfirm { confirm: String },
    AuthOk,
    AuthFail { reason: String },

    // ================= Agent → máy giáo viên =================
    Status { status: Status },
    FileDone { id: u64, ok: bool, path: String, error: String },
    Notice { text: String },
    /// Thu bài: Agent gửi từng file (dữ liệu đi bằng K_FILE với cùng id)
    CollectFile { task: String, id: u64, name: String, size: u64 },
    CollectEnd { task: String, files: u32, error: String },
    /// Kết quả cập nhật phần mềm do máy giáo viên đẩy xuống
    UpdateDone { ok: bool, version: String, error: String },
    /// Bài trắc nghiệm: `done` = đã nộp (hết giờ hoặc học sinh bấm Nộp bài)
    QuizAnswer { id: String, answers: Vec<i32>, user: String, #[serde(default)] done: bool },
    Pong,

    // ================= máy giáo viên → Agent =================
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
    /// Đặt chính sách chặn. `sig` là chữ ký của giáo viên để Helper quyền cao kiểm tra.
    SetPolicy { policy: Policy, ts: u64, sig: String },
    /// Đóng một ứng dụng đang mở (theo tên tiến trình)
    CloseApp { name: String },
    /// Cập nhật Agent: dữ liệu đi bằng K_FILE cùng id. `sig` ký trên auth::update_message(platform, version, sha256).
    UpdateBegin { id: u64, version: String, platform: String, size: u64, sha256: String, ts: u64, sig: String },
    UpdateEnd { id: u64 },
    /// Mở ứng dụng trên máy học sinh (tên chương trình, đường dẫn hoặc dòng lệnh)
    RunApp { cmd: String },
    /// Giao bài: tạo thư mục bài làm, hiện đề bài
    Assign { task: String, title: String, text: String },
    /// Thu bài: gửi các file trong thư mục bài làm về máy giáo viên
    Collect { task: String, title: String },
    QuizStart { quiz: Quiz },
    QuizStop { id: String },
    /// Đổi (hoặc xóa, nếu rỗng) mật khẩu phòng lưu trên máy học sinh. `sig` ký trên "password:<mật khẩu>".
    SetPairPassword { password: String, ts: u64, sig: String },
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
    write_frame(w, K_CTRL, &serde_json::to_vec(c)?).await?;
    w.flush().await?;
    Ok(())
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

/// Gói tìm máy giáo viên (UDP). Agent gửi broadcast, máy giáo viên trả lời trực tiếp.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Discover {
    pub lc: u32,
    pub kind: String, // "seek" (Agent hỏi) | "here" (máy giáo viên trả lời)
    #[serde(default)]
    pub room: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub teacher: String,
    #[serde(default)]
    pub name: String,
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

    #[test]
    fn quiz_hides_answers() {
        let q = QuizQuestion { q: "1+1?".into(), options: vec!["1".into(), "2".into()], answer: None };
        assert!(!serde_json::to_string(&q).unwrap().contains("answer"));
    }
}
