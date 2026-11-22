//! Công cụ thử cho nhà phát triển: đóng vai máy giáo viên, kết nối tới một Agent và gửi lệnh.
//!   cargo run --example thu_ket_noi -- <ip:cổng> <file khóa giáo viên> <file cần gửi>
use std::sync::Arc;

use lc_core::proto::{self, Ctrl, K_CTRL, K_FILE};
use lc_core::{auth, tls};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let key = auth::signing_key_from_b64(&std::fs::read_to_string(&a[2])?)?;
    let tcp = tokio::net::TcpStream::connect(&a[1]).await?;
    let (cfg, seen) = tls::client_config();
    let name = tokio_rustls::rustls::pki_types::ServerName::try_from("labcontrol-agent")?.to_owned();
    let s = tokio_rustls::TlsConnector::from(cfg).connect(name, tcp).await?;
    let fp = seen.lock().unwrap().clone().unwrap();
    let (mut rd, mut wr) = tokio::io::split(s);
    let (_, d) = proto::read_frame(&mut rd).await?;
    let Ctrl::Hello { nonce, info, .. } = proto::parse_ctrl(&d)? else { anyhow::bail!("lạ") };
    println!("Hello từ {} ({})", info.name, info.os);
    proto::write_ctrl(&mut wr, &Ctrl::Auth { teacher: auth::teacher_code(&key), teacher_name: "thử".into(),
                                            sig: auth::sign(&key, &nonce, &fp) }).await?;
    let (_, d) = proto::read_frame(&mut rd).await?;
    println!("Xác thực: {:?}", proto::parse_ctrl(&d)?);
    let data = Arc::new(std::fs::read(&a[3])?);
    let fname = std::path::Path::new(&a[3]).file_name().unwrap().to_string_lossy().to_string();
    proto::write_ctrl(&mut wr, &Ctrl::FileBegin { id: 7, name: format!("../../{fname}"), size: data.len() as u64 }).await?;
    for chunk in data.chunks(256 * 1024) {
        let mut c = 7u64.to_be_bytes().to_vec();
        c.extend_from_slice(chunk);
        proto::write_frame(&mut wr, K_FILE, &c).await?;
    }
    proto::write_ctrl(&mut wr, &Ctrl::FileEnd { id: 7 }).await?;
    proto::write_ctrl(&mut wr, &Ctrl::Message { text: "Các em mở Scratch, làm bài Pac-Man nhé!".into() }).await?;
    proto::write_ctrl(&mut wr, &Ctrl::GetStatus).await?;
    for _ in 0..4 {
        let (k, d) = proto::read_frame(&mut rd).await?;
        if k == K_CTRL {
            println!("{:?}", proto::parse_ctrl(&d)?);
        }
    }
    Ok(())
}
