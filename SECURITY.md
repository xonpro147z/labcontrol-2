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
| 1.0.x | Có |

## Thiết kế bảo mật hiện tại

| Hạng mục | Cách làm |
|---|---|
| Kết nối | TLS giữa Console và Agent. Agent tự sinh chứng chỉ mới mỗi lần chạy, không lưu khóa riêng ra đĩa. |
| Xác thực giáo viên | Chữ ký Ed25519 trên (nonce + dấu vân tay TLS của phiên). Máy học sinh chỉ lưu khóa công khai, nên đọc được file cấu hình cũng không giả làm giáo viên được, và kẻ đứng giữa (MITM) không chuyển tiếp được chữ ký. |
| Khóa riêng của giáo viên | Lưu trong thư mục cấu hình của người dùng trên máy giáo viên (`teacher.key`). Lộ file này thì phải đổi khóa và cài lại mã giáo viên trên máy học sinh. |
| Điều khiển từ xa | Chỉ nhận lệnh chuột/bàn phím khi giáo viên đang mở chế độ xem. Khi đó máy học sinh luôn hiện chỉ báo đỏ. |
| Nhận file | Tên file được làm sạch (không cho `../`), chỉ ghi vào thư mục `TuGiaoVien`, không tự chạy file. |
| Mở web | Chỉ chấp nhận địa chỉ `http://` và `https://`. |
| Ngoài phạm vi | Học sinh có quyền quản trị trên máy của mình có thể tắt hoặc gỡ Agent. |
