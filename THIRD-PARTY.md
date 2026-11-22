# Thư viện bên thứ ba

LabControl phát hành theo **AGPL-3.0-or-later**. Mọi thư viện đi kèm đều dùng giấy phép tương thích với AGPL-3.0. Danh sách dưới đây được kiểm tra ngày 22/11/2026 bằng `cargo metadata` và thông tin gói npm.

## Agent và Console (Rust)

| Giấy phép | Ghi chú |
|---|---|
| MIT, Apache-2.0, hoặc tùy chọn MIT/Apache | phần lớn thư viện (khoảng 540 crate cho Agent, 500 cho Console) |
| Unicode-3.0, ISC, BSD-3-Clause, Zlib, 0BSD, BSL-1.0, Unlicense, CC0 | một số crate nhỏ |
| MPL-2.0 | vài thư viện của Tauri/egui; tương thích GPL/AGPL theo điều 3.3 của MPL-2.0 |
| "MIT OR Apache-2.0 OR LGPL-2.1", "Apache-2.0 OR GPL-2.0" | dùng theo lựa chọn MIT/Apache |
| CDLA-Permissive-2.0 | `webpki-roots` |

Thư viện chính:
- `eframe`/`egui` (MIT/Apache): cửa sổ khóa máy, trình chiếu, tin nhắn
- `enigo` (MIT): điều khiển chuột/bàn phím
- `x11rb` (MIT/Apache)
- `rustls` + `ring` (Apache/ISC)
- `ed25519-dalek` (BSD-3)
- `jpeg-encoder` (MIT/Apache)
- `zune-jpeg` (MIT/Apache/Zlib)
- `rcgen` (MIT/Apache)
- `tauri` (MIT/Apache)

## Phông chữ đi kèm egui

| Phông | Giấy phép |
|---|---|
| Ubuntu Light | Ubuntu Font Licence 1.0 |
| Hack | MIT / Bitstream Vera |
| Noto Emoji | SIL OFL 1.1 |
| Emoji Icon Font | MIT |

Agent ưu tiên dùng phông của hệ điều hành (Segoe UI, Noto Sans, DejaVu Sans) để hiển thị tiếng Việt.

## Giao diện (npm)

| Thư viện | Giấy phép |
|---|---|
| React, React DOM | MIT |
| lucide-react | ISC |
| @tauri-apps/api, @tauri-apps/plugin-dialog, @tauri-apps/cli | Apache-2.0 OR MIT |
| Vite, TypeScript (chỉ dùng khi build) | MIT, Apache-2.0 |

## Tạo danh sách đầy đủ trước khi phát hành

```bash
cargo install cargo-about && (cd agent && cargo about generate about.hbs > THIRD-PARTY-agent.html)
(cd console && npx license-checker --production --summary)
```
