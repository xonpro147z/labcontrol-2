//! Phần lõi dùng chung giữa LabControl Agent (máy học sinh) và Console (máy giáo viên).
//!
//! Mô hình giống NetSupport School: máy giáo viên kết nối thẳng tới từng máy học sinh trong mạng LAN,
//! không cần máy chủ hay cơ sở dữ liệu.

pub mod auth;
pub mod capture;
pub mod img;
pub mod proto;
pub mod tls;

/// Cổng TCP Agent lắng nghe (có thể đổi bằng biến môi trường LABCONTROL_PORT khi thử nghiệm).
pub const AGENT_PORT: u16 = 47900;
/// Cổng UDP máy giáo viên nghe tín hiệu "tôi ở đây" của Agent.
pub const BEACON_PORT: u16 = 47901;
pub const PROTO_VERSION: u32 = 2;
