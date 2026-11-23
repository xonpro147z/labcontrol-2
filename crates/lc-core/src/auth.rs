//! Xác thực giáo viên bằng chữ ký Ed25519.
//!
//! Mỗi Console có một cặp khóa. Máy học sinh chỉ lưu **khóa công khai** của các giáo viên được phép
//! ("mã giáo viên" LCT-...), nên học sinh đọc được file cấu hình cũng không giả làm giáo viên được.
//! Chữ ký gắn với dấu vân tay chứng chỉ TLS của phiên, nên kẻ đứng giữa (MITM) không chuyển tiếp được.

use anyhow::{anyhow, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Kiểu khóa riêng của giáo viên (để Console không phải phụ thuộc trực tiếp ed25519-dalek)
pub type SigningKeyAlias = SigningKey;

pub const CODE_PREFIX: &str = "LCT-";

pub fn new_signing_key() -> SigningKey {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).expect("không lấy được số ngẫu nhiên");
    SigningKey::from_bytes(&seed)
}

pub fn signing_key_from_b64(s: &str) -> Result<SigningKey> {
    let b = B64.decode(s.trim())?;
    let arr: [u8; 32] = b.try_into().map_err(|_| anyhow!("khóa riêng không hợp lệ"))?;
    Ok(SigningKey::from_bytes(&arr))
}

pub fn signing_key_to_b64(k: &SigningKey) -> String {
    B64.encode(k.to_bytes())
}

/// "Mã giáo viên" để cài lên máy học sinh: LCT-<khóa công khai base64url>
pub fn teacher_code(k: &SigningKey) -> String {
    format!("{CODE_PREFIX}{}", B64.encode(k.verifying_key().to_bytes()))
}

pub fn parse_teacher_code(code: &str) -> Result<VerifyingKey> {
    let body = code.trim().strip_prefix(CODE_PREFIX).ok_or_else(|| anyhow!("Mã giáo viên phải bắt đầu bằng LCT-"))?;
    let b = B64.decode(body).map_err(|_| anyhow!("Mã giáo viên không hợp lệ"))?;
    let arr: [u8; 32] = b.try_into().map_err(|_| anyhow!("Mã giáo viên không hợp lệ"))?;
    VerifyingKey::from_bytes(&arr).map_err(|_| anyhow!("Mã giáo viên không hợp lệ"))
}

fn message(nonce: &str, tls_fp: &str) -> Vec<u8> {
    format!("labcontrol-v3-auth|{nonce}|{tls_fp}").into_bytes()
}

pub fn sign(k: &SigningKey, nonce: &str, tls_fp: &str) -> String {
    B64.encode(k.sign(&message(nonce, tls_fp)).to_bytes())
}

/// Kiểm tra chữ ký; `allowed` là danh sách mã giáo viên được phép điều khiển máy này.
pub fn verify(allowed: &[String], teacher_code_str: &str, sig: &str, nonce: &str, tls_fp: &str) -> bool {
    if !allowed.iter().any(|a| a.trim() == teacher_code_str.trim()) {
        return false;
    }
    let Ok(vk) = parse_teacher_code(teacher_code_str) else { return false };
    let Ok(sb) = B64.decode(sig) else { return false };
    let Ok(arr): Result<[u8; 64], _> = sb.try_into() else { return false };
    vk.verify(&message(nonce, tls_fp), &Signature::from_bytes(&arr)).is_ok()
}

pub fn random_nonce() -> String {
    let mut b = [0u8; 24];
    getrandom::getrandom(&mut b).expect("rng");
    B64.encode(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sign_verify() {
        let k = new_signing_key();
        let code = teacher_code(&k);
        let sig = sign(&k, "n1", "fp1");
        assert!(verify(&[code.clone()], &code, &sig, "n1", "fp1"));
        assert!(!verify(&[code.clone()], &code, &sig, "n1", "fp-khac"), "MITM với chứng chỉ khác phải bị từ chối");
        assert!(!verify(&[], &code, &sig, "n1", "fp1"), "giáo viên lạ phải bị từ chối");
        let other = new_signing_key();
        assert!(!verify(&[code.clone()], &code, &sign(&other, "n1", "fp1"), "n1", "fp1"));
    }
}

/// Chữ ký cho chính sách chặn (để Helper quyền cao trên máy học sinh kiểm tra, học sinh không giả được).
pub fn sign_policy(k: &SigningKey, policy_json: &str, ts: u64) -> String {
    B64.encode(k.sign(format!("labcontrol-v3-policy|{ts}|{policy_json}").as_bytes()).to_bytes())
}

pub fn verify_policy(allowed: &[String], policy_json: &str, ts: u64, sig: &str) -> bool {
    let Ok(sb) = B64.decode(sig) else { return false };
    let Ok(arr): Result<[u8; 64], _> = sb.try_into() else { return false };
    let sig = Signature::from_bytes(&arr);
    let msg = format!("labcontrol-v3-policy|{ts}|{policy_json}");
    allowed.iter().any(|code| parse_teacher_code(code).map(|vk| vk.verify(msg.as_bytes(), &sig).is_ok()).unwrap_or(false))
}
