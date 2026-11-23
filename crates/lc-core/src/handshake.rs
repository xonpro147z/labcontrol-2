//! Bắt tay phía máy giáo viên (dùng chung cho Console và công cụ thử).
//!
//! 1. Giáo viên → TeacherHello
//! 2. Agent → AgentHello { mode: key | pair | none }
//! 3. Giáo viên → Auth (ký nonce của Agent + dấu vân tay TLS)
//! 4. (mode pair) Agent → PairStart; giáo viên → PairReply; Agent → PairConfirm
//! 5. Giáo viên → AuthOk

use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use ed25519_dalek::SigningKey;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::proto::{self, AgentInfo, Ctrl, K_CTRL};
use crate::{auth, pair, PROTO_VERSION};

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Máy đã ghép từ trước
    Known,
    /// Vừa ghép bằng mật khẩu phòng
    Paired,
}

/// Lỗi bắt tay có kèm thông tin máy (để Console hiện "máy chưa ghép / sai mật khẩu")
#[derive(Debug)]
pub struct Rejected {
    pub info: Option<AgentInfo>,
    pub reason: String,
}

impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}
impl std::error::Error for Rejected {}

async fn next<R: AsyncRead + Unpin>(rd: &mut R) -> Result<Ctrl> {
    let (k, d) = tokio::time::timeout(Duration::from_secs(20), proto::read_frame(rd)).await
        .map_err(|_| anyhow!("máy học sinh không trả lời"))??;
    if k != K_CTRL {
        bail!("gói tin lạ khi bắt tay");
    }
    proto::parse_ctrl(&d)
}

pub struct TeacherSide<'a> {
    pub key: &'a SigningKey,
    pub name: &'a str,
    pub room: &'a str,
    /// Dấu vân tay chứng chỉ TLS của máy giáo viên
    pub fp: &'a str,
    /// Mật khẩu phòng; None = không nhận ghép máy mới
    pub password: Option<&'a str>,
}

pub async fn teacher<R, W>(rd: &mut R, wr: &mut W, t: &TeacherSide<'_>) -> std::result::Result<(AgentInfo, Outcome), Rejected>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut info: Option<AgentInfo> = None;
    let r: Result<Outcome> = async {
        let code = auth::teacher_code(t.key);
        proto::write_ctrl(wr, &Ctrl::TeacherHello { proto: PROTO_VERSION, nonce: auth::random_nonce(), teacher: code.clone(),
                                                    teacher_name: t.name.into(), room: t.room.into() }).await?;
        let (i, nonce, mode) = match next(rd).await? {
            Ctrl::AgentHello { info, nonce, mode } => (info, nonce, mode),
            _ => bail!("máy học sinh trả lời lạ"),
        };
        info = Some(i);
        proto::write_ctrl(wr, &Ctrl::Auth { sig: auth::sign(t.key, &nonce, t.fp) }).await?;
        let out = match mode.as_str() {
            "key" => Outcome::Known,
            "pair" => {
                let Some(pw) = t.password.filter(|p| !p.trim().is_empty()) else {
                    let _ = proto::write_ctrl(wr, &Ctrl::AuthFail { reason: "máy giáo viên chưa đặt mật khẩu phòng".into() }).await;
                    bail!("chưa ghép (máy giáo viên chưa đặt mật khẩu phòng)");
                };
                let peer = match next(rd).await? {
                    Ctrl::PairStart { msg } => msg,
                    Ctrl::AuthFail { reason } => bail!("{reason}"),
                    _ => bail!("máy học sinh trả lời lạ"),
                };
                let (p, msg) = pair::start(pw);
                let key = p.finish(&peer)?;
                proto::write_ctrl(wr, &Ctrl::PairReply { msg, confirm: pair::confirm(&key, "teacher", t.fp, &code) }).await?;
                match next(rd).await? {
                    Ctrl::PairConfirm { confirm } if pair::check(&key, "agent", t.fp, &code, &confirm) => {}
                    Ctrl::AuthFail { reason } => bail!("sai mật khẩu phòng ({reason})"),
                    _ => {
                        let _ = proto::write_ctrl(wr, &Ctrl::AuthFail { reason: "sai mật khẩu phòng".into() }).await;
                        bail!("sai mật khẩu phòng");
                    }
                }
                Outcome::Paired
            }
            _ => {
                let _ = next(rd).await;
                bail!("máy chưa có mật khẩu phòng");
            }
        };
        proto::write_ctrl(wr, &Ctrl::AuthOk).await?;
        Ok(out)
    }
    .await;
    match r {
        Ok(o) => Ok((info.clone().unwrap_or_default(), o)),
        Err(e) => Err(Rejected { info, reason: format!("{e:#}") }),
    }
}
