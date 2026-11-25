# Lộ trình và phát triển bền vững

LabControl là phần mềm tự do theo AGPL-3.0-or-later.

## Lộ trình

| Phiên bản | Nội dung | Trạng thái |
|---|---|---|
| 2.0 | Mô hình không cần máy chủ: xem cả lớp, xem lớn + điều khiển, trình chiếu, khóa, tin nhắn, mở web, gửi file, nguồn | Xong |
| 3.0 | Máy học sinh tự nối về, ghép bằng mật khẩu phòng; Helper dịch vụ hệ thống; sơ đồ phòng; chặn web/ứng dụng; chế độ thi; bài tập; trắc nghiệm; giao diện mới | **Xong**: đã thử trên Linux, chưa thử Windows thật |
| 3.1 | Helper mở lại Agent trong phiên học sinh khi bị tắt (học sinh khó tắt Agent hơn) | Cần làm |
| 3.1 | Trình chiếu qua multicast (một luồng cho cả phòng, máy giáo viên yếu vẫn chiếu được cho 40 máy) | Cần làm |
| 3.2 | Học sinh "giơ tay" xin trợ giúp, trò chuyện hai chiều | Cần làm |
| 3.2 | Trắc nghiệm: nhập đề từ Word/Excel, câu hỏi có hình, đảo câu/đáp án | Cần làm |
| 3.3 | Trình chiếu kèm âm thanh, trình chiếu màn hình một học sinh cho cả lớp | Cần làm |
| — | Chặn web theo danh sách cho phép (whitelist), lọc DNS | Cần làm |
| — | Wayland (Linux), nhiều màn hình, giao diện tiếng Anh | Cần làm |
| — | Tự cập nhật Console và Agent, bộ cài có ký số | Cần làm |

Mỗi mục nên được mở thành một Issue gắn nhãn `cần-giúp` hoặc `người-mới`.

## Trước khi khuyên trường khác dùng

1. **Thử trên máy thật:** Windows 10/11 và Linux Mint 21/22. Ghi kết quả thành bảng hỗ trợ trong README.
2. **Pilot ở một phòng máy 1–2 tuần.** Theo dõi:
   - độ trễ trình chiếu với cả phòng;
   - tải CPU trên máy học sinh cấu hình yếu;
   - Agent có tự chạy lại sau khi khởi động máy không.
3. **Ký số bộ cài:**
   - Cách miễn phí: [SignPath Foundation](https://signpath.org/) ký miễn phí cho dự án mã nguồn mở đủ điều kiện.
   - Cách trả phí: mua chứng thư OV/EV.
4. **Quyền riêng tư:** hệ thống cho phép giáo viên xem màn hình học sinh, có thể là trẻ vị thành niên. Nên:
   - soạn mẫu thông báo cho học sinh và phụ huynh;
   - đối chiếu với quy định bảo vệ dữ liệu cá nhân hiện hành của Việt Nam, nên nhờ người có chuyên môn pháp lý rà soát;
   - giữ nguyên chỉ báo "Giáo viên đang xem màn hình của bạn", **không** thêm chế độ xem ẩn.

## Tạo thu nhập mà vẫn giữ mã mở

| Cách | Ghi chú |
|---|---|
| Dịch vụ cài đặt, đào tạo cho trường | Phù hợp nhất lúc đầu |
| Gói hỗ trợ năm | Cam kết thời gian phản hồi, cập nhật, hỗ trợ qua Zalo/điện thoại |
| Bộ cài đã ký số, cập nhật tự động | Ai cũng tự build được, nhưng nhiều trường muốn bản dựng sẵn, đã kiểm thử |
| Tài trợ | GitHub Sponsors, Open Collective, sở/phòng giáo dục |

Dự án dùng DCO (người đóng góp giữ bản quyền phần của mình), nên không thể bán giấy phép thương mại riêng (dual licensing). Đây là lựa chọn có chủ đích để cộng đồng yên tâm rằng dự án luôn mở.
