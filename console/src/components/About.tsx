import { useState } from "react";
import { Code2, Copy } from "lucide-react";
import { Modal, toast } from "./ui";

export const VERSION = "3.0.2";
export const SOURCE_URL = "https://github.com/xonpro147z/labcontrol-2";

/** Liên kết "Mã nguồn mở" – đáp ứng điều 13 của AGPL-3.0. */
export function SourceLink({ dark }: { dark?: boolean }) {
  const [open, setOpen] = useState(false);
  return <>
    <button className="btn ghost sm" style={{ color: dark ? "#64748b" : undefined, padding: "0 4px", height: 22 }}
            onClick={() => setOpen(true)}><Code2 size={13} />Mã nguồn mở · AGPL-3.0</button>
    {open && <Modal title="Giới thiệu LabControl" onClose={() => setOpen(false)}
                    footer={<button className="btn primary" onClick={() => setOpen(false)}>Đóng</button>}>
      <p className="muted" style={{ margin: 0 }}>LabControl {VERSION} – phần mềm quản lý lớp học máy tính.</p>
      <p style={{ margin: 0 }}>Đây là <b>phần mềm tự do</b>: bạn được dùng, nghiên cứu, sửa đổi và chia sẻ theo Giấy phép
        Công cộng GNU Affero phiên bản 3 hoặc mới hơn (AGPL-3.0-or-later). Phần mềm được cung cấp <b>không kèm bất kỳ bảo đảm nào</b>.</p>
      <div className="field"><label>Mã nguồn</label>
        <div className="code-box">{SOURCE_URL}</div>
        <button className="btn sm" style={{ alignSelf: "flex-start" }}
                onClick={() => navigator.clipboard.writeText(SOURCE_URL).then(() => toast("Đã chép"))}><Copy size={13} />Chép địa chỉ</button>
      </div>
    </Modal>}
  </>;
}
