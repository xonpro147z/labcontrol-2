//! Phần lõi dùng chung giữa LabControl Agent (máy học sinh) và Console (máy giáo viên).
//!
//! Mô hình giống NetSupport School / iClassroom: máy học sinh tự tìm và nối về máy giáo viên trong mạng LAN,
//! không cần máy chủ hay cơ sở dữ liệu.

pub mod auth;
pub mod capture;
pub mod handshake;
pub mod img;
pub mod pair;
pub mod proto;
pub mod tls;

/// Cổng TCP máy giáo viên lắng nghe (máy học sinh nối vào). Chỉ máy giáo viên cần mở tường lửa.
pub const TEACHER_PORT: u16 = 47900;
/// Cổng UDP máy giáo viên trả lời "tôi ở đây" cho máy học sinh đang tìm.
pub const DISCOVER_PORT: u16 = 47901;
/// Cổng nội bộ (127.0.0.1) của Helper quyền cao trên máy học sinh.
pub const HELPER_PORT: u16 = 47903;
pub const PROTO_VERSION: u32 = 3;
