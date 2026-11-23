# LabControl

**Phần mềm tự do, mã nguồn mở để quản lý lớp học máy tính**, kiểu NetSupport School, chạy trên Windows và Linux Mint/Ubuntu.

[![Giấy phép: AGPL-3.0-or-later](https://img.shields.io/badge/gi%E1%BA%A5y%20ph%C3%A9p-AGPL--3.0--or--later-blue)](LICENSE)
[![Build](https://github.com/xonpro147z/labcontrol-2/actions/workflows/build.yml/badge.svg)](https://github.com/xonpro147z/labcontrol-2/actions)

Giống iClassroom / NetSupport School:

- **Máy học sinh tự tìm và tự nối về máy giáo viên** trong mạng LAN.
- Ghép lần đầu bằng **mật khẩu phòng**.
- **Không cần máy chủ hay cơ sở dữ liệu.** Chỉ máy giáo viên mở tường lửa.

Tính năng:

- **Lớp học:**
  - lưới màn hình cả lớp (cỡ S/M/L);
  - **sơ đồ phòng** kéo thả;
  - danh sách;
  - lọc, tìm kiếm, sắp xếp;
  - đặt tên hiển thị cho từng máy.
- **Xem lớn và điều khiển** chuột, bàn phím máy học sinh.
- **Trình chiếu** màn hình giáo viên, độ trễ thấp. Chỉ gửi vùng thay đổi, mạng 100 Mbps vẫn đủ cho cả phòng.
- **Lệnh cho máy học sinh:**
  - khóa / mở khóa, tin nhắn, mở web, gửi file;
  - tắt / khởi động lại / đăng xuất.
- **Giới hạn:**
  - chặn trang web;
  - chặn ứng dụng và công cụ hệ thống (cmd, PowerShell, Task Manager...);
  - đóng ứng dụng đang mở;
  - **chế độ thi**.
- **Bài tập:** giao bài, thu bài về máy giáo viên theo từng máy.
- **Trắc nghiệm:** soạn nhanh bằng văn bản, đếm giờ, chấm điểm tự động, xuất CSV.
- **Cập nhật:** máy giáo viên tự báo bản mới; đẩy bản cập nhật xuống cả phòng máy học sinh bằng một nút (có chữ ký số).

Hướng dẫn cài đặt và sử dụng: **[docs/HUONG-DAN.md](docs/HUONG-DAN.md)**

## Kiến trúc

```
LabControl (máy giáo viên)                        Máy học sinh
React + TypeScript + Tauri 2                       Agent (Rust, phiên của học sinh) + Helper (dịch vụ SYSTEM/root)
        │  ◄──── UDP 47901: "seek" ───────────────────  │   máy học sinh tìm máy giáo viên
        │  ───── "here" (mã giáo viên, cổng) ────────►  │
        │  ◄──── TLS, TCP 47900 ──────────────────────  │   máy học sinh nối vào, rồi nhận lệnh
```

**Ghép máy:**

- Lần đầu, hai bên chạy **SPAKE2** với mật khẩu phòng.
  - Mật khẩu không đi qua mạng.
  - Người nghe lén không dò được.
  - Mỗi lần đoán sai chỉ thử được một mật khẩu, và bị giới hạn tốc độ.
- Xác nhận khóa bằng HMAC gắn với dấu vân tay TLS (chống kẻ đứng giữa).
- Sau đó máy học sinh lưu khóa công khai Ed25519 của giáo viên (`LCT-...`). Các lần sau xác thực bằng chữ ký.

**Phân quyền trên máy học sinh:**

- **Agent** chạy dưới tài khoản học sinh để chụp màn hình và hiện cửa sổ.
- **Helper** (Windows Service / systemd):
  - giữ mật khẩu phòng (học sinh không đọc được);
  - chỉ sửa file hosts để chặn web khi lệnh có chữ ký của giáo viên đã ghép.

**Minh bạch:**

- Khi giáo viên xem hoặc điều khiển, máy học sinh luôn hiện dòng đỏ *"Giáo viên đang xem màn hình của bạn"*.
- Không ghi phím, không lưu ảnh.

| Thư mục | Nội dung |
|---|---|
| `crates/lc-core/` | Phần dùng chung: giao thức, bắt tay, ghép máy (SPAKE2), chụp màn hình (Windows GDI / X11), JPEG, mã hóa ô thay đổi, chữ ký, TLS |
| `agent/` | Agent + Helper máy học sinh: tìm và nối về máy giáo viên, điều khiển (enigo), chặn ứng dụng (sysinfo), cửa sổ khóa / trình chiếu / tin nhắn / trắc nghiệm (egui) |
| `console/` | LabControl máy giáo viên: giao diện React + phần Rust (trạm TLS, ghép máy, trình chiếu, bài tập, trắc nghiệm) |
| `deploy/` | Script cài / gỡ máy học sinh, mở tường lửa máy giáo viên |
| `.github/workflows/` | CI build Windows + Linux; gắn tag `v*` thì tạo trang Releases |

## Phát triển

```bash
# Kiểm thử
(cd crates/lc-core && cargo test) && (cd agent && cargo test)

# Máy giáo viên
cd console && npm install
npx tauri dev            # chạy thử
npx tauri build          # bộ cài

# Máy học sinh, chạy thử không cần cài (không có Helper thì Agent tự làm phần việc của Helper)
cd agent && cargo build
mkdir -p /tmp/hs1 && echo '{"password":"123456","name":"MAY-01"}' > /tmp/hs1/agent.json
LABCONTROL_AGENT_DIR=/tmp/hs1 LABCONTROL_HOSTS=/tmp/hs1/hosts LABCONTROL_ID=may-01 ./target/debug/labcontrol-agent run

# Giả lập máy giáo viên không cần giao diện (ghép bằng mật khẩu rồi gửi lệnh)
cargo run --example gia_lap_giao_vien -- /tmp/gv 123456 status assign:"Bài 1" collect:"Bài 1" quiz sleep:5 quizstop
```

Chạy nhiều Agent trên một máy để thử:

- Mỗi Agent một bộ `LABCONTROL_ID`, `LABCONTROL_AGENT_DIR` và `HOME` khác nhau.
- `LABCONTROL_SEEK_TO=127.0.0.1:47901` để tìm máy giáo viên trên cùng máy.

Muốn đóng góp? Đọc [CONTRIBUTING.md](CONTRIBUTING.md). Báo lỗ hổng bảo mật theo [SECURITY.md](SECURITY.md).

## Giấy phép

Copyright © 2026 Ngô Văn Hưng và những người đóng góp cho LabControl.

LabControl là phần mềm tự do: bạn có thể phân phối lại và/hoặc sửa đổi nó theo các điều khoản của **Giấy phép Công cộng GNU Affero** do Tổ chức Phần mềm Tự do công bố, phiên bản 3 hoặc (tùy bạn chọn) bất kỳ phiên bản nào mới hơn. Phần mềm được phân phối với hy vọng sẽ hữu ích nhưng **KHÔNG KÈM BẤT KỲ BẢO ĐẢM NÀO**. Xem [LICENSE](LICENSE).

**Nói ngắn gọn cho nhà trường:**
- Dùng, cài cho nhiều trường, thu phí dịch vụ cài đặt: được, miễn phí.
- Sửa mã rồi phân phối cho người khác: phải công khai mã đã sửa theo cùng giấy phép.
- Không ai được đóng mã LabControl lại để bán như phần mềm độc quyền.

Thư viện bên thứ ba: [THIRD-PARTY.md](THIRD-PARTY.md).
