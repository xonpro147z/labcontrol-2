# Chính sách bảo mật

LabControl chạy trên máy tính trong trường học và quản lý máy của học sinh, nên chúng tôi coi trọng mọi báo cáo về lỗ hổng.

## Báo lỗ hổng

- **Không** mở Issue công khai.
- Dùng tính năng **Security → Report a vulnerability** (GitHub Private Vulnerability Reporting) của kho mã, hoặc gửi email tới: `security@<điền-địa-chỉ-của-dự-án>`.
- Nên gửi kèm phiên bản bị ảnh hưởng, các bước tái hiện và mức ảnh hưởng (ví dụ: vượt quyền phạm vi phòng, giả mạo Agent, chiếm phiên đăng nhập).

Chúng tôi sẽ phản hồi trong vòng 7 ngày, công bố bản vá trước, sau đó mới công bố chi tiết. Người báo cáo được ghi công nếu muốn.

## Phiên bản được hỗ trợ

| Phiên bản | Nhận bản vá bảo mật |
|---|---|
| 3.0.x | Có |
| 2.0.x, 1.0.x | Không – hãy nâng cấp |

## Thiết kế bảo mật hiện tại

| Hạng mục | Cách làm |
|---|---|
| Kết nối | Máy học sinh nối TLS tới máy giáo viên (TCP 47900). Máy học sinh không mở cổng nào ra mạng. |
| Ghép máy lần đầu | SPAKE2 với mật khẩu phòng. Mật khẩu không đi qua mạng; người nghe lén không dò được ngoại tuyến. Khóa chung được xác nhận bằng HMAC gắn với dấu vân tay TLS của phiên và mã giáo viên, nên kẻ đứng giữa không chen vào được. |
| Dò mật khẩu | Helper trên máy học sinh chỉ cho thử 1 lần/giây, khóa 5 phút sau 10 lần sai. Nên dùng mật khẩu phòng dài hơn 6 ký tự nếu mạng không tin cậy. |
| Các lần sau | Chữ ký Ed25519 của giáo viên trên (nonce + dấu vân tay TLS). Máy học sinh chỉ lưu khóa công khai. |
| Mật khẩu phòng trên máy học sinh | Lưu trong `ProgramData\LabControl\agent.json` (`/etc/labcontrol`), chỉ SYSTEM/Administrators (root) đọc được. Agent của học sinh hỏi Helper qua `127.0.0.1:47903`, không đọc được mật khẩu. |
| Chặn web, đổi mật khẩu từ xa | Helper chỉ làm khi lệnh có chữ ký của giáo viên đã ghép, có dấu thời gian tăng dần (chống phát lại). |
| Giới hạn tự hết hạn | Chặn web / ứng dụng / chế độ thi tự bỏ sau 3 phút nếu máy giáo viên ngừng gửi lại, để máy không bị kẹt. |
| Khóa riêng của giáo viên | `teacher.key` trong thư mục cấu hình của người dùng trên máy giáo viên. Lộ file này thì xóa nó (LabControl tạo khóa mới) và chạy `labcontrol-agent forget` trên máy học sinh. |
| Điều khiển từ xa | Chỉ nhận lệnh chuột/bàn phím khi giáo viên đang mở chế độ xem. Khi đó máy học sinh luôn hiện chỉ báo đỏ. |
| Nhận / nộp file | Tên file được làm sạch (không cho `../`, đường dẫn tuyệt đối). Máy học sinh chỉ ghi vào `TuGiaoVien`; máy giáo viên chỉ ghi vào thư mục bài nộp. Không tự chạy file. |
| Trắc nghiệm | Đáp án không bao giờ được gửi xuống máy học sinh. |
| Mở web | Chỉ chấp nhận địa chỉ `http://` và `https://`. |
| Ngoài phạm vi | Học sinh có quyền quản trị trên máy có thể tắt hoặc gỡ LabControl. Chặn web bằng file hosts không chặn được VPN/proxy. |
