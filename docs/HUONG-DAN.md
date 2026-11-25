# Hướng dẫn sử dụng LabControl 3

LabControl quản lý phòng máy tính trong mạng LAN:

- **Máy học sinh tự tìm và tự nối về máy giáo viên** trong cùng mạng LAN.
- Ghép máy lần đầu bằng **mật khẩu phòng**.
- Không cần máy chủ, cơ sở dữ liệu hay Internet.
- Chỉ máy giáo viên cần mở tường lửa.

| Máy | Cài gì |
|---|---|
| Máy giáo viên | **LabControl**: bộ cài `LabControl_x.y.z_x64-setup.exe` (Windows) hoặc `.deb` / `.AppImage` (Linux Mint/Ubuntu) |
| Máy học sinh | Gói `labcontrol-agent-windows-x64.zip` (có `labcontrol-agent.exe`, `cai-may-hoc-sinh.cmd`, `go-may-hoc-sinh.cmd`) hoặc `labcontrol-agent-linux-x64.tar.gz` |

Tất cả có ở trang **Releases** của dự án.

## 1. Cài trên máy giáo viên

1. Chạy bộ cài LabControl.
   - Windows chưa ký số nên sẽ báo *"Windows protected your PC"*: bấm **More info → Run anyway**.
   - Bộ cài tự mở tường lửa cho cổng TCP 47900 và UDP 47901.
   - **macOS** (máy giáo viên): tải `LabControl_…_universal.dmg`, mở ra rồi kéo LabControl vào **Applications**.
     Lần đầu mở, macOS báo không xác định nhà phát triển: chuột phải vào LabControl → **Open** → **Open**.
     Trên macOS 15 trở lên: vào **System Settings → Privacy & Security**, kéo xuống và bấm **Open Anyway**.
     Nếu báo *"bị hỏng"* (damaged), mở Terminal và chạy `xattr -cr /Applications/LabControl.app`.
     Khi macOS hỏi cho phép kết nối mạng đến, chọn **Allow**.
     Máy giáo viên Mac điều khiển được máy học sinh Windows và Linux; máy học sinh chạy macOS thì chưa hỗ trợ.
2. Mở LabControl.
3. Chọn **Cài đặt → Phòng & ghép máy** và xem **mật khẩu phòng** (lần đầu là 6 chữ số ngẫu nhiên, đổi được).
   - Nếu mạng có nhiều phòng máy, đặt thêm **tên phòng** (ví dụ `P.Tin 1`).

## 2. Cài trên máy học sinh (mỗi máy một lần)

**Windows:**

1. Giải nén gói Agent vào một thư mục (USB, thư mục chia sẻ...).
2. Chuột phải `cai-may-hoc-sinh.cmd` → *Run as administrator*.
3. Nhập **mật khẩu phòng** và tên phòng (nếu có).
4. Đăng xuất rồi đăng nhập lại, hoặc khởi động lại máy.

Máy học sinh sẽ tự hiện trên màn hình giáo viên sau vài giây. Máy học sinh **không cần mở tường lửa**.

Cài hàng loạt (GPO, script, phần mềm nhân bản máy):
```
cai-may-hoc-sinh.cmd <mật-khẩu-phòng> [tên-phòng] [ip-máy-giáo-viên]
```

**Linux Mint / Ubuntu (phiên X11):**
```
sudo bash install-agent-linux.sh <mật-khẩu-phòng> [tên-phòng]
```

Bộ cài cài hai phần:

| Phần | Chạy dưới | Việc |
|---|---|---|
| **Agent** | tài khoản học sinh, tự chạy khi đăng nhập | chụp màn hình, điều khiển, màn hình khóa, bài tập, trắc nghiệm, chặn ứng dụng |
| **Helper** | dịch vụ hệ thống (SYSTEM / root) | giữ mật khẩu phòng, chặn trang web; học sinh không tắt được |

### Ghép máy hoạt động thế nào

1. Lần đầu, máy học sinh tìm thấy máy giáo viên và chứng minh hai bên cùng biết mật khẩu phòng.
   - Dùng giao thức SPAKE2: mật khẩu không bao giờ được gửi qua mạng.
   - Người nghe lén không dò được mật khẩu.
2. Máy học sinh ghi nhớ máy giáo viên này.
3. Từ đó máy tự vào lớp, kể cả khi giáo viên đổi mật khẩu phòng.

Nhiều giáo viên dùng chung phòng máy:

- Mỗi giáo viên đặt **cùng mật khẩu phòng** trên LabControl của mình.
- Máy học sinh sẽ ghép với từng máy giáo viên ở lần đầu gặp.

Cần đổi máy giáo viên (cài lại Windows, thay máy):

- Máy giáo viên mới chỉ cần dùng **cùng mật khẩu phòng**; máy học sinh tự ghép lại.
- Muốn máy học sinh quên các máy giáo viên cũ, chạy `labcontrol-agent forget` bằng quyền quản trị.

## 3. Màn hình Lớp học

Bố cục: thanh biểu tượng bên trái, **thanh công cụ** phía trên, **thanh lọc** và vùng hiển thị máy học sinh.

### Chọn máy

- Bấm vào ô để chọn hoặc bỏ chọn; ô vuông đầu thanh lọc chọn tất cả.
- **Không chọn máy nào thì lệnh áp dụng cho cả lớp.** Góc phải thanh công cụ luôn ghi rõ lệnh áp dụng cho bao nhiêu máy.

### Thanh lọc

- Tab **Tất cả / Trực tuyến / Ngoại tuyến / Chưa ghép**.
- Ô tìm theo tên máy, tên học sinh, ứng dụng đang dùng, địa chỉ IP.
- Sắp xếp theo tên máy, theo học sinh, hoặc máy trực tuyến trước.
- Ba kiểu xem:
  - **Lưới màn hình**, cỡ ảnh S/M/L.
  - **Sơ đồ phòng**: kéo thả máy vào chỗ ngồi giống phòng thật, thả lên máy khác để đổi chỗ. Nút **Xếp tự động theo tên** xếp nhanh.
  - **Danh sách**: bảng tên máy, học sinh, ứng dụng đang dùng, IP, phiên bản.
- Chuột phải vào máy:
  - Xem / điều khiển, khóa, nhắn tin.
  - **Đặt tên hiển thị** (ví dụ tên học sinh ngồi máy đó).
  - Gỡ máy ngoại tuyến khỏi danh sách.

### Thanh công cụ

| Nút | Việc |
|---|---|
| Khóa máy ▾ | Khóa màn hình kèm lời nhắn (chặn bàn phím, chuột) / Mở khóa |
| Trình chiếu ▾ | Chiếu màn hình giáo viên: toàn màn hình (khóa bàn phím, chuột; học sinh không thoát ra được) hoặc trong cửa sổ. Máy vào lớp muộn tự nhận trình chiếu. |
| Tin nhắn | Hiện hộp tin nhắn trên máy học sinh |
| Gửi file ▾ | Gửi file hoặc cả thư mục, hoặc **kéo file từ máy tính thả vào cửa sổ LabControl**. Học sinh nhận ở thư mục `TuGiaoVien` trên màn hình nền, có thông báo khi nhận xong. Tiến độ hiện ở góc dưới phải. |
| Mở web | Mở trang web trên trình duyệt của học sinh |
| Mở ứng dụng | Mở chương trình trên máy học sinh: chọn nhanh (Word, Excel, Paint, LibreOffice...) hoặc nhập tên / đường dẫn, ví dụ `C:\Program Files (x86)\Scratch 3\Scratch 3.exe`. Thêm ứng dụng hay dùng trong Cài đặt › Danh sách chặn. |
| Bài tập | Giao bài (xem mục 4) |
| Trắc nghiệm | Sang trang Bài & kiểm tra |
| Chặn web ▾ | Chọn nhanh trang trong danh sách hoặc thêm tên miền; chặn cả `www.` và `m.` |
| Chặn ứng dụng ▾ | Chặn công cụ hệ thống (cmd, PowerShell, Task Manager, Registry...) và/hoặc ứng dụng trong danh sách: mở lên là bị đóng ngay |
| Đóng ứng dụng | Đóng một ứng dụng đang mở. Gợi ý các ứng dụng học sinh đang dùng. |
| Chế độ thi | Hiện dải "Đang trong giờ kiểm tra"; đóng trình duyệt, ứng dụng nhắn tin, phần mềm điều khiển từ xa và công cụ hệ thống |
| Tắt tiếng | Tắt loa máy học sinh; học sinh tự bật lại sẽ bị tắt lại ngay. Bấm lần nữa để bật tiếng. Linux cần `pactl` hoặc `amixer` |
| Nguồn ▾ | Tắt máy / Khởi động lại / Đăng xuất |
| Bỏ giới hạn | Bỏ mọi chặn web, chặn ứng dụng, chế độ thi |

Chặn web, chặn ứng dụng và chế độ thi:

- Đang chọn máy thì chỉ áp dụng cho các máy đó (giới hạn riêng); không chọn máy nào thì áp dụng cho cả lớp và xóa mọi giới hạn riêng.
- Ví dụ cả lớp đang bị chặn web mà một máy cần mở web: **chuột phải vào máy đó → Bỏ giới hạn cho máy này**.
  Muốn máy đó bị chặn lại như cả lớp: chuột phải → **Dùng giới hạn chung của lớp**.

- Giới hạn chung của lớp áp dụng cả cho máy vào lớp sau.
- Máy giáo viên gửi lại mỗi phút. Nếu LabControl trên máy giáo viên bị tắt ngang hoặc mất mạng, máy học sinh **tự bỏ giới hạn sau 3 phút**, để không bị kẹt.

Biểu tượng trên ô máy:

- 🔒 đang khóa
- 👁 đang được xem
- trình chiếu
- chế độ thi
- đang chặn web
- khiên gạch chéo: máy chưa có Helper nên không chặn web được (cài lại bằng `cai-may-hoc-sinh.cmd`)

### Xem lớn và điều khiển

- Bấm đúp vào máy, hoặc nút mũi tên trên ô.
- Bấm **Điều khiển** để dùng chuột và bàn phím của mình trên máy học sinh.

**Quyền riêng tư:**

- Khi giáo viên xem lớn hoặc điều khiển, máy học sinh luôn hiện dòng đỏ *"Giáo viên đang xem màn hình của bạn"*.
- LabControl không ghi phím và không lưu ảnh màn hình.

## 4. Bài & kiểm tra

### Bài tập

1. Nhập tên bài và đề bài, bấm **Giao cho cả lớp**. Hoặc dùng nút **Bài tập** trên thanh công cụ để chỉ giao cho máy đang chọn.
2. Máy học sinh tạo thư mục `BaiLam\<tên bài>` trên màn hình nền, kèm `DE-BAI.txt`, mở thư mục và hiện đề bài.
3. Học sinh lưu bài vào thư mục đó.
4. Bấm **Thu bài**. File được lưu trên máy giáo viên tại `Tài liệu\LabControl\BaiNop\<tên bài>\<tên máy - học sinh>\`.
   - Thu lại lần nữa sẽ ghi đè bản cũ.
   - Giới hạn 300 file / 500 MB mỗi máy.

### Trắc nghiệm

1. Soạn đề trong ô **Câu hỏi**:
   - Mỗi câu cách nhau **một dòng trống**.
   - Dòng đầu là câu hỏi, các dòng sau là đáp án.
   - Đánh dấu `*` trước đáp án đúng.
   ```
   Thủ đô của Việt Nam là?
   *Hà Nội
   Huế

   2 + 3 = ?
   4
   *5
   ```
2. Đặt thời gian (0 = không giới hạn), bấm **Xem trước** rồi **Phát đề cho cả lớp**.
3. Máy học sinh hiện cửa sổ làm bài có đồng hồ đếm ngược.
   - **Không gửi đáp án xuống máy học sinh.**
   - Hết giờ thì tự nộp.
   - Học sinh tắt cửa sổ thì cửa sổ tự mở lại, giữ nguyên câu đã chọn.
4. Bảng kết quả cập nhật trực tiếp: số câu đã làm, số câu đúng, điểm thang 10, điểm trung bình.
5. Bấm **Thu bài & kết thúc** để nộp bài của mọi máy. Bấm **Kết quả (CSV)** để mở thư mục `Tài liệu\LabControl\TracNghiem`; file CSV mở được bằng Excel.

## 5. Nhật ký

Ghi lại trong buổi học:

- máy vào / thoát lớp;
- máy nộp bài;
- ứng dụng bị chặn trên máy nào;
- máy nhập sai mật khẩu phòng...

## 6. Cài đặt

| Mục | Ý nghĩa |
|---|---|
| Tên giáo viên, tên phòng | **Nhiều phòng máy chung một mạng thì mỗi phòng phải đặt tên phòng riêng** trên máy giáo viên và khi cài máy học sinh (ví dụ `P.Tin1`, `P.Tin2`). Máy học sinh có tên phòng chỉ vào lớp của máy giáo viên cùng tên phòng. Không phân biệt hoa thường, khoảng trắng. |
| Giao diện & ngôn ngữ | Tiếng Việt / English (máy học sinh đổi theo), chế độ sáng/tối/theo hệ thống, màu nhấn, hình nền, độ kính trong suốt, cỡ giao diện, bo góc. Chỉnh là thấy ngay; bấm Hủy để trả lại như cũ. |
| Mật khẩu phòng | Đổi mật khẩu thì máy đang trực tuyến được cập nhật luôn. Máy đã ghép vẫn vào lớp bình thường. |
| Nhận máy mới | Tắt khi đã đủ máy, để không ai ghép thêm máy lạ vào lớp |
| Cài máy học sinh | Lệnh cài sẵn mật khẩu phòng để chép |
| Danh sách chặn | Các trang web / ứng dụng hiện thành nút chọn nhanh |
| Hình ảnh & tốc độ | Chu kỳ cập nhật ảnh, độ nét, khung hình/giây khi trình chiếu |
| Nền tối | Nút cuối thanh bên trái |

## 7. Cập nhật phần mềm

**Máy giáo viên:** LabControl tự kiểm tra bản mới trên GitHub khi mở và mỗi 12 giờ (cần Internet). Khi có bản mới, màn hình
Lớp học hiện dải *"Đã có LabControl x.y.z"* → bấm **Xem & tải về**, tải bộ cài và cài đè lên bản cũ (cấu hình, mật khẩu phòng,
danh sách máy được giữ nguyên). Kiểm tra bằng tay: **Cài đặt → Giới thiệu → Kiểm tra bản mới**.

**Máy học sinh – cập nhật từ xa, không cần đi từng máy:**
1. Cập nhật máy giáo viên trước (bộ cài máy giáo viên mang sẵn phần mềm máy học sinh cùng phiên bản).
2. Trên màn hình Lớp học bấm **"N máy dùng bản cũ · Cập nhật"** (hoặc chuột phải một máy → Cập nhật phần mềm,
   hoặc **Cài đặt → Cài máy học sinh → Cập nhật máy học sinh từ xa**).
3. Máy học sinh nhận bản mới, tự cài và khởi động lại LabControl trong khoảng 10–30 giây. Học sinh không cần làm gì.

- Chỉ máy đang trực tuyến được cập nhật; máy đang tắt sẽ hiện "bản cũ" khi vào lớp lần sau.
- Gói cập nhật có chữ ký của máy giáo viên đã ghép và được kiểm tra mã băm, nên máy học sinh không nhận gói từ nơi khác.
- Máy học sinh đang dùng **3.0.0 hoặc 3.0.1** phải cài lại bằng tay **một lần** (chạy lại `cai-may-hoc-sinh.cmd` /
  `install-agent-linux.sh` của bản mới, cùng mật khẩu phòng). Từ 3.0.2 trở đi cập nhật từ xa được.

## 8. Cổng mạng

| Cổng | Máy | Dùng cho |
|---|---|---|
| TCP 47900 | giáo viên | máy học sinh nối vào (mã hóa TLS) |
| UDP 47901 | giáo viên | máy học sinh tìm máy giáo viên |
| TCP 47903 | học sinh, chỉ `127.0.0.1` | Agent nói chuyện với Helper trên cùng máy |

Máy học sinh không mở cổng nào ra mạng.

## 9. Lỗi thường gặp

| Hiện tượng | Cách xử lý |
|---|---|
| Không thấy máy học sinh nào | Kiểm tra: (1) máy học sinh đã đăng nhập lại sau khi cài; (2) cùng mạng LAN; (3) tường lửa máy giáo viên đã mở (chạy `mo-tuong-lua-may-giao-vien.cmd`); (4) tên phòng hai bên giống nhau. |
| Mạng khác dải / Wi-Fi cách ly, máy học sinh không tìm thấy | Cài máy học sinh kèm IP máy giáo viên: `cai-may-hoc-sinh.cmd <mật-khẩu> "" 192.168.1.10`. Nên đặt IP tĩnh cho máy giáo viên. |
| Tab **Chưa ghép** có máy, lý do "sai mật khẩu phòng" | Cài lại máy đó với mật khẩu đúng. Sau 10 lần sai, máy học sinh tạm khóa thử 5 phút. |
| "Đang tắt nhận máy mới" | Bật lại **Nhận máy mới** trong Cài đặt. |
| Báo không mở được cổng 47900 | Đang mở hai cửa sổ LabControl, hoặc phần mềm khác dùng cổng này. |
| Chặn web không có tác dụng | Máy chưa có Helper: cài lại bằng `cai-may-hoc-sinh.cmd`. Trình duyệt có thể còn nhớ trang cũ vài phút; tắt hẳn trình duyệt rồi mở lại. |
| Linux: ảnh đen hoặc không điều khiển được | Đăng nhập bằng phiên **X11**. Wayland chưa được hỗ trợ. |

Nhật ký để gửi kèm khi báo lỗi:

- Agent: `%TEMP%\labcontrol-agent.log` (Windows), `/tmp/labcontrol-agent.log` (Linux)
- Helper: `labcontrol-helper.log` trong `C:\Windows\SystemTemp` hoặc `C:\Windows\Temp` (Windows), `/tmp/labcontrol-helper.log` (Linux)

Lệnh quản trị trên máy học sinh:

- `labcontrol-agent status`: xem cấu hình.
- `labcontrol-agent set-password <mới>`: đổi mật khẩu phòng.
- `labcontrol-agent forget`: quên các máy giáo viên đã ghép.
- `labcontrol-agent uninstall` hoặc `go-may-hoc-sinh.cmd`: gỡ.

## 10. Nâng cấp từ bản 2

- Cài LabControl 3 trên máy giáo viên (giữ nguyên khóa giáo viên cũ).
- Cài lại máy học sinh bằng `cai-may-hoc-sinh.cmd` với mật khẩu phòng.
- Các máy đã nhận mã giáo viên ở bản 2 vào lớp ngay, không cần ghép lại.
- Bản 3 không còn mở cổng 47900 trên máy học sinh. Bộ cài xóa luật tường lửa cũ.

## 11. Giới hạn hiện tại

- **Agent chạy dưới tài khoản học sinh.**
  - Học sinh dùng tài khoản quản trị có thể tắt Agent; nên cho học sinh dùng tài khoản thường.
  - Helper là dịch vụ hệ thống nên tài khoản thường không tắt được.
- **Chặn web dùng file hosts.**
  - Không chặn được truy cập qua VPN, proxy hoặc DNS-over-HTTPS cài sẵn trong trình duyệt.
  - Chế độ thi đóng hẳn trình duyệt.
- **Ctrl+Alt+Del vẫn dùng được** khi máy bị khóa, vì Windows không cho phần mềm chặn tổ hợp này.
- **Trình chiếu:** chỉ chụp màn hình chính, chưa có âm thanh.
- **Bản Windows** được build tự động nhưng cần thử trên máy thật trước khi dùng cho cả trường.
