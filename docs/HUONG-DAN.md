# Hướng dẫn sử dụng LabControl 2

LabControl hoạt động giống NetSupport School: **máy giáo viên kết nối thẳng tới các máy học sinh** trong cùng mạng LAN. Không cần máy chủ, không cần cơ sở dữ liệu, không cần Internet.

| Máy | Cài gì |
|---|---|
| Máy giáo viên | **LabControl Console**: bộ cài `LabControl_x.y.z_x64-setup.exe` (Windows) hoặc `.deb` (Linux Mint/Ubuntu) |
| Máy học sinh | **LabControl Agent**: `labcontrol-agent.exe` + `cai-agent-windows.cmd` (Windows) hoặc `labcontrol-agent` + `install-agent-linux.sh` (Linux) |

Tất cả có ở trang **Releases** của dự án.

## 1. Cài trên máy giáo viên

1. Chạy bộ cài Console.
   - Windows chưa ký số nên sẽ báo *"Windows protected your PC"*: bấm **More info → Run anyway**.
2. Lần đầu mở Console, Windows có thể hỏi cho phép qua tường lửa: chọn **Allow**.
   - Hoặc chuột phải `mo-tuong-lua-may-giao-vien.cmd` → *Run as administrator*.
3. Vào **Cài đặt**, chép **Mã giáo viên** (dạng `LCT-...`).

Mã này là khóa công khai của máy giáo viên. Máy học sinh chỉ nhận lệnh từ máy có mã đã cài sẵn. Học sinh có biết mã cũng không dùng được để điều khiển máy khác.

## 2. Cài trên máy học sinh

**Windows:**

1. Chép `labcontrol-agent.exe` và `cai-agent-windows.cmd` vào **cùng một thư mục**.
2. Chuột phải `cai-agent-windows.cmd` → *Run as administrator* → dán mã giáo viên.
3. Đăng xuất rồi đăng nhập lại (hoặc khởi động lại máy).

Cài hàng loạt (GPO, script) thì truyền mã và tên phòng thẳng trong lệnh:
```
cai-agent-windows.cmd LCT-xxxxxxxx P201
```

**Linux Mint / Ubuntu (phiên X11):**
```
sudo bash install-agent-linux.sh LCT-xxxxxxxx P201
```
rồi đăng xuất và đăng nhập lại.

Agent tự chạy mỗi khi có người đăng nhập và tự báo cho máy giáo viên. Máy học sinh sẽ hiện trên Console sau vài giây.

**Nhiều giáo viên dùng chung phòng:** trên máy học sinh chạy thêm lệnh sau (bằng quyền quản trị) với mã của từng giáo viên:
```
labcontrol-agent add-teacher LCT-...
```

## 3. Sử dụng

| Việc | Cách làm |
|---|---|
| Xem cả lớp | Màn hình chính hiện lưới ảnh màn hình học sinh, cập nhật mỗi 2 giây. Thanh trượt góc phải phóng to hoặc thu nhỏ ô. |
| Chọn máy | Bấm vào ô. **Không chọn máy nào thì lệnh áp dụng cho cả lớp.** |
| Xem lớn, điều khiển | Bấm đúp vào một máy. Bấm **Điều khiển** để dùng chuột và bàn phím của mình trên máy học sinh, bấm **Thôi điều khiển** để dừng. |
| Trình chiếu | Bấm **Trình chiếu**. Màn hình giáo viên hiện trên máy học sinh, toàn màn hình hoặc trong cửa sổ. Bấm **Dừng trình chiếu** để kết thúc. |
| Khóa / Mở khóa | Màn hình học sinh chuyển sang màn khóa kèm lời nhắn; bàn phím và chuột bị chặn. |
| Tin nhắn | Hiện hộp tin nhắn trên máy học sinh. |
| Mở web | Mở trang web trên trình duyệt của học sinh. |
| Gửi file | Chọn file trên máy giáo viên. File được lưu vào thư mục `TuGiaoVien` trên Desktop của học sinh. |
| Nguồn | Tắt máy, khởi động lại hoặc đăng xuất máy học sinh. |

**Quyền riêng tư:** khi giáo viên xem lớn hoặc điều khiển, góc trên màn hình học sinh luôn hiện dòng đỏ *"Giáo viên đang xem màn hình của bạn"*. Agent không ghi phím và không lưu ảnh màn hình.

## 4. Cài đặt trên Console

| Mục | Ý nghĩa |
|---|---|
| Tên giáo viên | Hiện trong nhật ký của máy học sinh |
| Chỉ hiện máy của phòng | Dùng khi nhiều phòng chung một mạng. Phải trùng với tên phòng đặt lúc cài Agent. |
| Cập nhật ảnh thu nhỏ | Mạng yếu hoặc lớp đông thì tăng lên 3–5 giây |
| Trình chiếu: khung hình/giây, chất lượng | Máy giáo viên yếu thì giảm khung hình/giây (8–10) |
| Thêm máy theo IP | Khi mạng chặn tín hiệu tìm máy (khác lớp mạng, Wi-Fi tách biệt) |

## 5. Cổng mạng

| Cổng | Máy | Dùng cho |
|---|---|---|
| TCP 47900 | học sinh | Console kết nối tới (bộ cài Agent tự mở tường lửa) |
| UDP 47901 | giáo viên | nhận tín hiệu tìm máy |

## 6. Lỗi thường gặp

| Hiện tượng | Cách xử lý |
|---|---|
| Không thấy máy học sinh nào | Kiểm tra máy học sinh đã đăng nhập lại sau khi cài, máy giáo viên đã mở tường lửa UDP 47901, hai máy cùng mạng. Hoặc thêm máy theo IP. |
| Ô báo *"Máy không nhận mã giáo viên này"* | Agent trên máy đó được cài bằng mã của giáo viên khác. Chạy `labcontrol-agent add-teacher <mã của bạn>`. |
| Ô báo *"Mất kết nối"* | Máy tắt, chưa đăng nhập, hoặc Agent bị tắt. Agent tự chạy lại ở lần đăng nhập sau. |
| Linux: ảnh đen hoặc không điều khiển được | Đăng nhập bằng phiên **X11**. Wayland chưa được hỗ trợ. |
| Nhật ký Agent | `%TEMP%\labcontrol-agent.log` (Windows), `/tmp/labcontrol-agent.log` (Linux) |

## 7. Giới hạn hiện tại

- Agent chạy dưới tài khoản học sinh. Học sinh dùng tài khoản quản trị có thể tắt Agent bằng Task Manager, vì vậy nên cho học sinh dùng tài khoản thường.
  - Nếu học sinh tắt riêng cửa sổ khóa hoặc cửa sổ trình chiếu, Agent tự mở lại trong vòng 1 giây.
- Ctrl+Alt+Del vẫn dùng được khi máy bị khóa, vì Windows không cho phần mềm chặn tổ hợp này.
- Chỉ chụp và trình chiếu màn hình chính; chưa có âm thanh khi trình chiếu.
- Bản Windows được build tự động nhưng cần thử trên máy thật trước khi dùng cho cả trường.
