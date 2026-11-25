# Nhật ký thay đổi

Định dạng theo [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/), đánh số phiên bản theo [SemVer](https://semver.org/lang/vi/).

## [3.0.4] – 2026-11-25

### Sửa
- Ô màn hình học sinh bị kéo cao bằng cả cửa sổ khi máy học sinh đang mở ứng dụng: dòng tên ứng dụng trong ô dùng trùng
  tên lớp CSS `app` với khung chính của giao diện. Đã đổi tên và kiểm tra với 44 máy trên Chromium (WebView2) và WebKitGTK.

## [3.0.3] – 2026-11-25

### Sửa
- Lưới màn hình học sinh bị kéo dài rất cao, ảnh dồn xuống đáy ô trên một số máy Linux (WebKitGTK cũ không hỗ trợ đúng `aspect-ratio`).
  Ô màn hình nay dùng khung cố định 16:10, hiển thị đúng trên mọi phiên bản.

## [3.0.2] – 2026-11-23

### Thêm
- **Báo bản mới cho máy giáo viên**: LabControl tự kiểm tra GitHub khi mở và mỗi 12 giờ, hiện dải "Đã có LabControl x.y.z – Xem & tải về".
  Có nút "Kiểm tra bản mới" trong Cài đặt › Giới thiệu.
- **Cập nhật máy học sinh từ xa**: bộ cài máy giáo viên mang theo Agent cùng phiên bản (Windows + Linux).
  Nút "N máy dùng bản cũ · Cập nhật" gửi bản mới xuống; Helper (quyền hệ thống) kiểm tra chữ ký của giáo viên
  và mã băm SHA-256, cài đặt rồi khởi động lại Agent. Có thể chọn file Agent khác trong Cài đặt › Cài máy học sinh.

### Sửa
- Lệnh lạ từ phiên bản khác không còn làm ngắt kết nối.

### Lưu ý
- Máy học sinh đang dùng 3.0.0 / 3.0.1 phải cài lại bằng tay **một lần** lên 3.0.2; từ đó về sau cập nhật từ xa được.

## [3.0.1] – 2026-11-23

### Thêm
- **Mở ứng dụng** trên máy học sinh: nhập tên chương trình, đường dẫn hoặc lệnh; có nút chọn nhanh (Word, Excel, PowerPoint,
  Paint, LibreOffice...) và danh sách ứng dụng riêng trong Cài đặt › Danh sách chặn.

### Sửa
- Trình chiếu toàn màn hình: học sinh không còn thoát ra màn hình chính được (chặn bàn phím, chuột như màn hình khóa;
  bị thu nhỏ thì tự phóng lại).
- Ô màn hình học sinh bị kéo dài xuống dưới khi các máy có tỉ lệ màn hình khác nhau.

## [3.0.0] – 2026-11-23

Đổi cách kết nối giống iClassroom: **máy học sinh tự tìm và tự nối về máy giáo viên**, ghép bằng **mật khẩu phòng**.
Giao diện máy giáo viên làm lại hoàn toàn.

### Kết nối và ghép máy
- Máy học sinh gửi tín hiệu tìm máy giáo viên (UDP 47901), máy giáo viên trả lời; máy học sinh nối TLS về cổng 47900 của máy giáo viên.
  Máy học sinh không mở cổng nào, không cần mở tường lửa.
- Ghép lần đầu bằng mật khẩu phòng (SPAKE2 + xác nhận HMAC gắn với phiên TLS), sau đó dùng chữ ký Ed25519. Không còn phải chép "mã giáo viên".
- Tùy chọn tên phòng (nhiều phòng chung mạng), địa chỉ IP máy giáo viên cố định (mạng chặn quảng bá), tắt nhận máy mới, đổi mật khẩu phòng từ xa.
- Tab "Chưa ghép" hiện máy nhập sai mật khẩu. Giới hạn 10 lần sai / 5 phút trên mỗi máy học sinh.
- Dịch vụ **Helper** (Windows Service / systemd) giữ mật khẩu phòng và thực hiện chặn web.

### Giao diện máy giáo viên
- Bố cục mới: thanh biểu tượng bên trái (Lớp học, Bài & kiểm tra, Nhật ký, Cài đặt, Nền tối), thanh công cụ lớn có menu thả xuống.
- Thanh lọc: chọn tất cả, tab Tất cả / Trực tuyến / Ngoại tuyến / Chưa ghép, tìm kiếm, sắp xếp, cỡ ảnh S/M/L.
- Ba kiểu xem: lưới màn hình, **sơ đồ phòng** kéo thả (xếp tự động theo tên), danh sách. Đặt tên hiển thị cho máy; máy ngoại tuyến vẫn hiện.
- Cài đặt dạng hộp thoại nhiều mục; nền tối; màu nhận diện xanh ngọc.

### Thêm
- Chặn trang web (file hosts, qua Helper), chặn công cụ hệ thống, chặn ứng dụng theo tên, đóng ứng dụng đang mở.
- Chế độ thi: dải "Đang trong giờ kiểm tra", đóng trình duyệt / ứng dụng nhắn tin / điều khiển từ xa / công cụ hệ thống.
- Giới hạn tự hết hạn sau 3 phút nếu máy giáo viên ngừng gửi lại (Console bị tắt ngang).
- Bài tập: giao bài (thư mục BaiLam + DE-BAI.txt), thu bài về máy giáo viên theo từng máy.
- Trắc nghiệm: soạn nhanh bằng văn bản, đếm giờ, tự nộp khi hết giờ, chấm điểm trực tiếp, xuất CSV. Không gửi đáp án xuống máy học sinh.
- Nhật ký buổi học.
- Bộ cài máy giáo viên tự mở tường lửa; script `cai-may-hoc-sinh.cmd`, `go-may-hoc-sinh.cmd`.

### Thay đổi không tương thích
- Giao thức 3 không nói chuyện được với Agent / Console bản 2. Cài lại máy học sinh bằng `cai-may-hoc-sinh.cmd`.
- Bỏ lệnh `install --teacher`, `add-teacher`; thay bằng `install --password`, `set-password`, `forget`.

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
