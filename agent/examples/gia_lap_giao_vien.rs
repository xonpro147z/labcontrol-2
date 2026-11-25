//! Công cụ thử cho nhà phát triển: đóng vai máy giáo viên tối giản.
//! Chờ Agent tự nối về, ghép bằng mật khẩu phòng, rồi chạy lần lượt các lệnh trên dòng lệnh.
//!
//!   cargo run --example gia_lap_giao_vien -- <thư mục dữ liệu> <mật khẩu phòng> [lệnh...]
//!
//! Lệnh: status | thumb | msg:<chữ> | lock | unlock | assign:<tên> | collect:<tên> | quiz | quizstop
//!       | tools | exam | web:<miền> | clear | close:<tên tiến trình> | run:<ứng dụng> | setpw:<mật khẩu> | sleep:<giây>
use std::time::{SystemTime, UNIX_EPOCH};

use lc_core::handshake::{self, TeacherSide};
use lc_core::proto::{self, Ctrl, Discover, Policy, Quiz, QuizQuestion, K_CTRL, K_FILE, K_THUMB};
use lc_core::{auth, tls, DISCOVER_PORT, PROTO_VERSION, TEACHER_PORT};
use tokio::io::AsyncWriteExt;

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let dir = std::path::PathBuf::from(&a[1]);
    std::fs::create_dir_all(&dir)?;
    let kp = dir.join("teacher.key");
    let key = match std::fs::read_to_string(&kp) {
        Ok(s) => auth::signing_key_from_b64(s.trim())?,
        Err(_) => {
            let k = auth::new_signing_key();
            std::fs::write(&kp, auth::signing_key_to_b64(&k))?;
            k
        }
    };
    let code = auth::teacher_code(&key);
    let (srv, fp) = tls::server_config(&dir)?;
    let port: u16 = std::env::var("LABCONTROL_TEACHER_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(TEACHER_PORT);
    // trả lời "seek"
    let udp = tokio::net::UdpSocket::bind(("0.0.0.0", DISCOVER_PORT)).await?;
    let code2 = code.clone();
    tokio::spawn(async move {
        let mut b = [0u8; 2048];
        while let Ok((n, from)) = udp.recv_from(&mut b).await {
            if let Ok(d) = serde_json::from_slice::<Discover>(&b[..n]) {
                if d.kind == "seek" {
                    let r = Discover { lc: PROTO_VERSION, kind: "here".into(), room: std::env::var("LABCONTROL_ROOM").unwrap_or_default(), port, teacher: code2.clone(), name: "Thử".into() };
                    let _ = udp.send_to(&serde_json::to_vec(&r).unwrap(), from).await;
                }
            }
        }
    });
    let l = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Chờ máy học sinh ở cổng {port}...");
    let (tcp, addr) = l.accept().await?;
    let s = tokio_rustls::TlsAcceptor::from(srv).accept(tcp).await?;
    let (mut rd, mut wr) = tokio::io::split(s);
    let side = TeacherSide { key: &key, name: "Thử", room: "", fp: &fp, password: Some(&a[2]) };
    match handshake::teacher(&mut rd, &mut wr, &side).await {
        Ok((info, o)) => println!("BẮT TAY OK {addr}: {} ({}) {:?}", info.name, info.user, o),
        Err(e) => {
            println!("BẮT TAY LỖI: {e} {:?}", e.info.as_ref().map(|i| &i.name));
            return Ok(());
        }
    }
    let reader = tokio::spawn(async move {
        let mut files: std::collections::HashMap<u64, (String, Vec<u8>)> = Default::default();
        while let Ok((k, d)) = proto::read_frame(&mut rd).await {
            match k {
                K_CTRL => {
                    let c = proto::parse_ctrl(&d).unwrap();
                    if let Ctrl::CollectFile { id, name, .. } = &c {
                        files.insert(*id, (name.clone(), vec![]));
                    }
                    if let Ctrl::CollectEnd { .. } = &c {
                        for (n, b) in files.values() {
                            println!("  BÀI NỘP {n}: {} byte: {:?}", b.len(), String::from_utf8_lossy(&b[..b.len().min(40)]));
                        }
                    }
                    println!("<- {c:?}");
                }
                K_THUMB => println!("<- ảnh {} byte", d.len()),
                K_FILE => {
                    let id = u64::from_be_bytes(d[..8].try_into().unwrap());
                    if let Some(f) = files.get_mut(&id) {
                        f.1.extend_from_slice(&d[8..]);
                    }
                }
                _ => {}
            }
        }
        println!("Máy học sinh ngắt");
    });
    let pol = |p: Policy, key: &lc_core::auth::SigningKeyAlias| {
        let ts = now();
        let sig = auth::sign_policy(key, &serde_json::to_string(&p).unwrap(), ts);
        Ctrl::SetPolicy { policy: p, ts, sig }
    };
    for cmd in &a[3..] {
        let (c, arg) = cmd.split_once(':').unwrap_or((cmd, ""));
        let m = match c {
            "status" => Ctrl::GetStatus,
            "thumb" => Ctrl::GetThumb { width: 320, quality: 60 },
            "msg" => Ctrl::Message { text: arg.into() },
            "lock" => Ctrl::Lock { message: "Nhìn lên bảng".into() },
            "unlock" => Ctrl::Unlock,
            "assign" => Ctrl::Assign { task: "t1".into(), title: arg.into(), text: "Viết chương trình tính tổng 1..n".into() },
            "collect" => Ctrl::Collect { task: "t1".into(), title: arg.into() },
            "quiz" => Ctrl::QuizStart { quiz: Quiz { id: "q1".into(), title: "Kiểm tra 5 phút".into(), minutes: 1, questions: vec![
                QuizQuestion { q: "Thủ đô của Việt Nam?".into(), options: vec!["Hà Nội".into(), "Huế".into(), "Đà Nẵng".into()], answer: None },
                QuizQuestion { q: "2 + 3 = ?".into(), options: vec!["4".into(), "5".into(), "6".into(), "7".into()], answer: None },
            ] } },
            "quizstop" => Ctrl::QuizStop { id: "q1".into() },
            "tools" => pol(Policy { block_tools: true, ..Default::default() }, &key),
            "exam" => pol(Policy { exam: true, ..Default::default() }, &key),
            "web" => pol(Policy { block_web: vec![arg.into()], ..Default::default() }, &key),
            "clear" => pol(Policy::default(), &key),
            "close" => Ctrl::CloseApp { name: arg.into() },
            "run" => Ctrl::RunApp { cmd: arg.into() },
            "showfs" => Ctrl::ShowStart { fullscreen: true },
            "showstop" => Ctrl::ShowStop,
            "sendf" => {
                // gửi file cục bộ dưới tên "ThuMuc/<tên file>"
                let data = std::fs::read(arg)?;
                let name = format!("ThuMuc/con/{}", std::path::Path::new(arg).file_name().unwrap().to_string_lossy());
                proto::write_ctrl(&mut wr, &Ctrl::FileBegin { id: 99, name, size: data.len() as u64 }).await?;
                for c in data.chunks(65536) {
                    let mut b = 99u64.to_be_bytes().to_vec();
                    b.extend_from_slice(c);
                    proto::write_frame(&mut wr, K_FILE, &b).await?;
                }
                Ctrl::FileEnd { id: 99 }
            }
            "setpw" => {
                let ts = now();
                Ctrl::SetPairPassword { password: arg.into(), ts, sig: auth::sign_policy(&key, &format!("password:{arg}"), ts) }
            }
            "sleep" => {
                tokio::time::sleep(std::time::Duration::from_secs_f32(arg.parse().unwrap_or(1.0))).await;
                continue;
            }
            _ => continue,
        };
        println!("-> {cmd}");
        proto::write_ctrl(&mut wr, &m).await?;
    }
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    wr.shutdown().await.ok();
    reader.abort();
    Ok(())
}
