# LabControl

**Phần mềm tự do, mã nguồn mở để quản lý lớp học máy tính**, kiểu NetSupport School, chạy trên Windows và Linux Mint/Ubuntu.

[![Giấy phép: AGPL-3.0-or-later](https://img.shields.io/badge/gi%E1%BA%A5y%20ph%C3%A9p-AGPL--3.0--or--later-blue)](LICENSE)
[![Build](https://github.com/xonpro147z/labcontrol-2/actions/workflows/build.yml/badge.svg)](https://github.com/xonpro147z/labcontrol-2/actions)

Máy giáo viên kết nối thẳng tới các máy học sinh trong mạng LAN, **không cần máy chủ hay cơ sở dữ liệu**:

- **Xem màn hình cả lớp** dạng lưới; bấm đúp để xem lớn.
- **Điều khiển chuột, bàn phím** máy học sinh từ xa.
- **Trình chiếu màn hình giáo viên** xuống cả lớp, độ trễ thấp. Chỉ gửi những vùng màn hình thay đổi, nên mạng 100 Mbps vẫn đủ cho cả phòng.
- **Khóa / mở khóa** máy, **gửi tin nhắn**, **mở trang web**, **gửi file**, **tắt / khởi động lại / đăng xuất**.

Hướng dẫn cài đặt và sử dụng: **[docs/HUONG-DAN.md](docs/HUONG-DAN.md)**

## Kiến trúc

```
LabControl Console (máy giáo viên)                LabControl Agent (mỗi máy học sinh)
React + TypeScript + Tauri 2                       Rust, chạy trong phiên đăng nhập của học sinh
        │  ◄──── UDP 47901: "tôi ở đây" ──────────────  │
        │  ───── TLS, TCP 47900 ─────────────────────►  │  ảnh màn hình, điều khiển, lệnh, file, trình chiếu
```

- **Xác thực:** Console có cặp khóa Ed25519. Máy học sinh chỉ lưu khóa công khai ("mã giáo viên" `LCT-...`) và chỉ nhận lệnh có chữ ký hợp lệ. Chữ ký gắn với chứng chỉ TLS của phiên, nên kẻ đứng giữa (MITM) không chuyển tiếp được.
- **Minh bạch:** khi giáo viên xem hoặc điều khiển, máy học sinh luôn hiện dòng đỏ *"Giáo viên đang xem màn hình của bạn"*. Không ghi phím, không lưu ảnh.

| Thư mục | Nội dung |
|---|---|
| `crates/lc-core/` | Phần dùng chung: giao thức, chụp màn hình (Windows GDI / X11), JPEG, mã hóa ô thay đổi, xác thực, TLS |
| `agent/` | Agent máy học sinh: máy chủ TLS, tín hiệu tìm máy, điều khiển (enigo), cửa sổ khóa/trình chiếu/tin nhắn (egui) |
| `console/` | Console máy giáo viên: giao diện React + phần Rust (tìm máy, kết nối, trình chiếu) |
| `deploy/` | Script cài Agent và mở tường lửa |
| `.github/workflows/` | CI build Windows + Linux; gắn tag `v*` thì tạo trang Releases |

## Phát triển

```bash
# Agent (máy học sinh)
cd agent && cargo build --release
sudo ./target/release/labcontrol-agent install --teacher LCT-...    # hoặc chạy thử: labcontrol-agent run

# Console (máy giáo viên)
cd console && npm install
npx tauri dev            # chạy thử
npx tauri build          # bộ cài

# Kiểm thử
(cd crates/lc-core && cargo test) && (cd agent && cargo test)
# Thử kết nối không cần giao diện:
cd agent && cargo run --example thu_ket_noi -- 127.0.0.1:47900 <file teacher.key> <file gửi thử>
```

Chạy nhiều Agent trên một máy để thử: đặt `LABCONTROL_PORT`, `LABCONTROL_AGENT_DIR` và `DISPLAY` khác nhau cho từng Agent.

Muốn đóng góp? Đọc [CONTRIBUTING.md](CONTRIBUTING.md). Báo lỗ hổng bảo mật theo [SECURITY.md](SECURITY.md).

## Giấy phép

Copyright © 2026 Ngô Văn Hưng và những người đóng góp cho LabControl.

LabControl là phần mềm tự do: bạn có thể phân phối lại và/hoặc sửa đổi nó theo các điều khoản của **Giấy phép Công cộng GNU Affero** do Tổ chức Phần mềm Tự do công bố, phiên bản 3 hoặc (tùy bạn chọn) bất kỳ phiên bản nào mới hơn. Phần mềm được phân phối với hy vọng sẽ hữu ích nhưng **KHÔNG KÈM BẤT KỲ BẢO ĐẢM NÀO**. Xem [LICENSE](LICENSE).

**Nói ngắn gọn cho nhà trường:**
- Dùng, cài cho nhiều trường, thu phí dịch vụ cài đặt: được, miễn phí.
- Sửa mã rồi phân phối cho người khác: phải công khai mã đã sửa theo cùng giấy phép.
- Không ai được đóng mã LabControl lại để bán như phần mềm độc quyền.

Thư viện bên thứ ba: [THIRD-PARTY.md](THIRD-PARTY.md).
