# Đóng góp cho LabControl

Cảm ơn bạn quan tâm. Mọi đóng góp đều được chào đón: báo lỗi, đề xuất tính năng, sửa tài liệu, dịch, kiểm thử trên máy thật, và viết mã.

## Bắt đầu nhanh

| Phần | Lệnh |
|---|---|
| Lõi + Agent | `(cd crates/lc-core && cargo test) && (cd agent && cargo test && cargo build --release)` |
| Console | `cd console && npm install && npx tauri dev` |
| Giả lập máy giáo viên | `cd agent && cargo run --example gia_lap_giao_vien -- <thư mục> <mật khẩu phòng> status quiz ...` |
| Nhiều Agent một máy | đặt `LABCONTROL_PORT`, `LABCONTROL_AGENT_DIR`, `DISPLAY` khác nhau |

Xem `README.md` để hiểu kiến trúc. Thư mục `docs/` có hướng dẫn triển khai và lộ trình.

## Quy trình

1. Với thay đổi lớn, mở *Issue* trước để thống nhất hướng làm.
2. Fork, tạo nhánh (`sua-loi-heartbeat`, `tinh-nang-wake-on-lan`…), mỗi PR chỉ làm một việc.
3. Thêm hoặc sửa kiểm thử cho phần thay đổi; CI phải xanh.
4. Mọi commit phải có dòng **Signed-off-by** (xem bên dưới).
5. Mô tả trong PR: thay đổi gì, vì sao, đã thử trên hệ điều hành nào.

## Chứng nhận nguồn gốc (DCO)

Dự án dùng [Developer Certificate of Origin](https://developercertificate.org/) thay cho CLA. Khi commit bằng `git commit -s`, bạn xác nhận mình có quyền gửi đoạn mã này theo giấy phép AGPL-3.0-or-later của dự án:

```
Signed-off-by: Nguyễn Văn A <email@example.com>
```

Bản quyền phần đóng góp vẫn thuộc về bạn. Vì vậy, sau này dự án **không thể** tự đổi sang giấy phép khác nếu không được mọi người đóng góp đồng ý. Đây là cam kết rằng LabControl sẽ luôn là phần mềm tự do.

## Quy ước

- **Ngôn ngữ:** giao diện, thông báo lỗi và tài liệu viết tiếng Việt có dấu. Tên biến và hàm trong mã có thể dùng tiếng Việt không dấu hoặc tiếng Anh, miễn nhất quán trong từng tệp.
- **TypeScript:** chế độ `strict`, không dùng `any` khi tránh được.
- **Rust:** chạy `cargo fmt` và `cargo clippy`.
- **Bảo mật trước hết:**
  - Lệnh mới: thêm vào `Ctrl` trong `crates/lc-core/src/proto.rs`, xử lý ở `agent/src/session.rs`, và cho phép trong `send_ctrl` của Console (`console/src-tauri/src/lib.rs`).
  - Không thêm chức năng giám sát ẩn. Mọi tính năng xem hoặc điều khiển máy học sinh phải hiện chỉ báo rõ ràng trên máy học sinh (mục 9 của kế hoạch dự án).
- **Giao thức:** thay đổi không tương thích phải tăng `PROTO_VERSION`.

## Báo lỗi bảo mật

**Đừng** mở Issue công khai cho lỗ hổng bảo mật. Hãy làm theo `SECURITY.md`.

## Ứng xử

Tôn trọng, kiên nhẫn và thiện chí với mọi người, nhất là các thầy cô mới làm quen với mã nguồn mở. Xem `CODE_OF_CONDUCT.md`.
