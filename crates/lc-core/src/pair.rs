//! Ghép máy học sinh với máy giáo viên bằng **mật khẩu phòng**.
//!
//! Dùng SPAKE2 (giao thức trao đổi khóa có mật khẩu): kẻ nghe lén hay giả mạo một phía không thể đoán
//! mật khẩu ngoại tuyến; mỗi lần thử sai chỉ thử được đúng một mật khẩu. Khóa chung được xác nhận
//! bằng HMAC gắn với dấu vân tay TLS của phiên (chống kẻ đứng giữa) và mã giáo viên.
//!
//! Sau khi ghép xong, Agent lưu mã giáo viên (khóa công khai) và các lần sau dùng chữ ký Ed25519,
//! không cần mật khẩu nữa.

use anyhow::{anyhow, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};

const ID: &[u8] = b"labcontrol-v3-room";

pub struct Pairing(Spake2<Ed25519Group>);

pub fn normalize(pw: &str) -> String {
    pw.trim().to_string()
}

/// Bắt đầu ghép. Trả về (trạng thái, thông điệp gửi đi).
pub fn start(password: &str) -> (Pairing, String) {
    let (s, msg) = Spake2::<Ed25519Group>::start_symmetric(&Password::new(normalize(password).as_bytes()), &Identity::new(ID));
    (Pairing(s), B64.encode(msg))
}

impl Pairing {
    pub fn finish(self, peer_msg: &str) -> Result<Vec<u8>> {
        let m = B64.decode(peer_msg).map_err(|_| anyhow!("thông điệp ghép không hợp lệ"))?;
        self.0.finish(&m).map_err(|_| anyhow!("ghép máy thất bại"))
    }
}

pub fn confirm(key: &[u8], role: &str, tls_fp: &str, teacher: &str) -> String {
    let mut m = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("hmac");
    m.update(format!("labcontrol-v3-confirm|{role}|{tls_fp}|{teacher}").as_bytes());
    B64.encode(m.finalize().into_bytes())
}

pub fn check(key: &[u8], role: &str, tls_fp: &str, teacher: &str, got: &str) -> bool {
    let want = confirm(key, role, tls_fp, teacher);
    want.len() == got.len() && want.bytes().zip(got.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pair_ok_and_fail() {
        let (a, ma) = start("123456");
        let (b, mb) = start("123456");
        let ka = a.finish(&mb).unwrap();
        let kb = b.finish(&ma).unwrap();
        assert_eq!(ka, kb);
        let c = confirm(&ka, "teacher", "fp", "LCT-x");
        assert!(check(&kb, "teacher", "fp", "LCT-x", &c));
        assert!(!check(&kb, "teacher", "fp-khac", "LCT-x", &c), "MITM với chứng chỉ khác phải thất bại");

        let (a, ma) = start("123456");
        let (b, mb) = start("654321");
        let ka = a.finish(&mb).unwrap();
        let kb = b.finish(&ma).unwrap();
        assert_ne!(ka, kb, "sai mật khẩu thì khóa khác nhau");
    }
}
