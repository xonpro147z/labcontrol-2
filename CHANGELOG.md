# Nhật ký thay đổi

Định dạng theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/), đánh số phiên bản theo [SemVer](https://semver.org/lang/vi/).

## [2.0.0] – 2026-11-22

Đổi sang mô hình **kiểu NetSupport School**: máy giáo viên kết nối thẳng tới máy học sinh trong mạng LAN.
Bỏ hẳn Server và PostgreSQL.

### Thêm
- Xem màn hình cả lớp dạng lưới (phóng to/thu nhỏ ô); xem lớn từng máy.
- Điều khiển chuột, bàn phím máy học sinh từ xa.
- Trình chiếu màn hình giáo viên (toàn màn hình hoặc cửa sổ), chỉ gửi vùng thay đổi, máy vào muộn tự nhận trình chiếu.
- Khóa/mở khóa (chặn bàn phím, chuột), tin nhắn, mở trang web, gửi file, tắt máy/khởi động lại/đăng xuất.
- Xác thực bằng chữ ký Ed25519 gắn với phiên TLS; máy học sinh chỉ lưu khóa công khai của giáo viên.
- Chỉ báo "Giáo viên đang xem màn hình của bạn" trên máy học sinh.

### Bỏ
- Server FastAPI, PostgreSQL, đăng nhập nhiều người dùng, nhật ký tập trung (có trong bản 1.0.0).

## [1.0.0] – 2026-11-22

Bản phát hành đầu tiên, tương ứng giai đoạn 1–3 của kế hoạch LabControl.

### Có trong bản này

- **Server (FastAPI + PostgreSQL):**
  - đăng nhập, phân quyền theo vai trò và phạm vi phòng (Giáo viên, Quản lý phòng, Quản trị viên);
  - audit log, xuất CSV;
  - quản lý phòng và máy, đăng ký Agent bằng khóa có hạn dùng;
  - WebSocket thời gian thực, cảnh báo CPU/RAM/ổ đĩa/mất kết nối;
  - TLS tự ký có ghim dấu vân tay, migration Alembic.
- **Agent (Rust):**
  - heartbeat, thông tin phần cứng, người dùng đang đăng nhập;
  - lệnh trong danh sách cho phép;
  - chạy như Windows Service hoặc systemd, tự kết nối lại với thời gian chờ tăng dần.
- **Console (React + Tauri 2):**
  - giao diện Hybrid Modern: Dashboard, phòng máy dạng lưới và sơ đồ kéo thả, chi tiết máy, cảnh báo, nhật ký, cài đặt;
  - chạy dạng ứng dụng Desktop hoặc web.
- Phát hành theo giấy phép **AGPL-3.0-or-later**.
