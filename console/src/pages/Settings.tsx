import { useState } from "react";
import { Copy, Plus, Trash2 } from "lucide-react";
import { api, reload, setSettingsLocal, useStore, type Settings } from "../api";
import { Topbar } from "../App";
import { errText, toast } from "../components/ui";

export function SettingsPage() {
  const { teacherCode, settings } = useStore();
  const [f, setF] = useState<Settings>(settings!);
  const [ip, setIp] = useState("");
  const copy = (s: string) => navigator.clipboard.writeText(s).then(() => toast("Đã chép"), () => toast("Không chép được", "error"));
  const win = `cai-agent-windows.cmd ${teacherCode}${f.room ? ` ${f.room}` : ""}`;
  const lin = `sudo ./labcontrol-agent install --teacher ${teacherCode}${f.room ? ` --room "${f.room}"` : ""}`;

  async function save(next = f) {
    try { await api.setSettings(next); setSettingsLocal(next); await reload(); toast("Đã lưu cài đặt"); } catch (e) { toast(errText(e), "error"); }
  }

  return <>
    <Topbar title="Cài đặt" crumb="Hệ thống" />
    <div className="page">
      <div className="card">
        <div className="card-h"><h2>Mã giáo viên của máy này</h2></div>
        <div className="card-b col" style={{ gap: 12 }}>
          <div className="muted small">Máy học sinh chỉ cho phép những máy giáo viên có mã đã cài sẵn kết nối tới. Mã này là
            khóa công khai, học sinh có biết cũng không dùng được để điều khiển máy khác.</div>
          <div className="code-box">{teacherCode}</div>
          <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => copy(teacherCode)}><Copy size={13} />Chép mã</button>
          <div className="field"><label>Cài Agent trên máy học sinh Windows (chuột phải → Run as administrator, hoặc chạy trong cmd quản trị)</label>
            <div className="code-box">{win}</div>
            <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => copy(win)}><Copy size={13} />Chép lệnh</button></div>
          <div className="field"><label>Cài Agent trên Linux Mint / Ubuntu</label>
            <div className="code-box">{lin}</div>
            <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => copy(lin)}><Copy size={13} />Chép lệnh</button></div>
          <div className="small faint">Nhiều giáo viên dùng chung một phòng: trên máy học sinh chạy thêm
            <span className="mono"> labcontrol-agent add-teacher LCT-...</span> với mã của từng giáo viên.</div>
        </div>
      </div>

      <div className="two">
        <div className="card">
          <div className="card-h"><h2>Lớp học</h2></div>
          <div className="card-b col" style={{ gap: 14 }}>
            <div className="field"><label>Tên giáo viên</label>
              <input className="input" value={f.teacher_name} onChange={(e) => setF({ ...f, teacher_name: e.target.value })} /></div>
            <div className="field"><label>Chỉ hiện máy của phòng</label>
              <input className="input" value={f.room} placeholder="để trống = mọi máy tìm thấy" onChange={(e) => setF({ ...f, room: e.target.value })} />
              <span className="hint">Dùng khi nhiều phòng chung một mạng. Đặt cùng tên phòng khi cài Agent.</span></div>
            <div className="grid2">
              <div className="field"><label>Cập nhật ảnh thu nhỏ (giây)</label>
                <input className="input" type="number" min={0.5} step={0.5} value={f.thumb_seconds}
                       onChange={(e) => setF({ ...f, thumb_seconds: +e.target.value })} /></div>
              <div className="field"><label>Độ nét ảnh thu nhỏ (px)</label>
                <input className="input" type="number" min={160} max={800} step={40} value={f.thumb_width}
                       onChange={(e) => setF({ ...f, thumb_width: +e.target.value })} /></div>
            </div>
            <div className="grid2">
              <div className="field"><label>Trình chiếu: khung hình/giây</label>
                <input className="input" type="number" min={2} max={30} value={f.show_fps} onChange={(e) => setF({ ...f, show_fps: +e.target.value })} /></div>
              <div className="field"><label>Trình chiếu: chất lượng (30–90)</label>
                <input className="input" type="number" min={30} max={90} value={f.show_quality} onChange={(e) => setF({ ...f, show_quality: +e.target.value })} /></div>
            </div>
            <button className="btn primary" style={{ alignSelf: "flex-start" }} onClick={() => save()}>Lưu cài đặt</button>
          </div>
        </div>
        <div className="card">
          <div className="card-h"><h2>Thêm máy theo địa chỉ IP</h2></div>
          <div className="card-b col" style={{ gap: 10 }}>
            <div className="small muted">Bình thường máy học sinh tự hiện. Chỉ cần thêm tay khi mạng chặn tín hiệu tìm máy
              (khác lớp mạng, Wi-Fi tách biệt...).</div>
            <div className="row">
              <input className="input" value={ip} placeholder="192.168.1.21" onChange={(e) => setIp(e.target.value)} />
              <button className="btn" onClick={async () => {
                if (!ip.trim()) return;
                try {
                  await api.addManual(ip.trim());
                  const next = { ...f, manual: [...new Set([...f.manual, ip.trim()])] };
                  setF(next); await save(next); setIp("");
                } catch (e) { toast(errText(e), "error"); }
              }}><Plus size={14} />Thêm</button>
            </div>
            {f.manual.map((m) => <div key={m} className="row"><span className="mono grow">{m}</span>
              <button className="btn sm danger" onClick={async () => {
                const next = { ...f, manual: f.manual.filter((x) => x !== m) };
                setF(next); await save(next);
                const id = m.includes(":") ? m : `${m}:47900`;
                api.remove(id);
              }}><Trash2 size={13} /></button></div>)}
          </div>
        </div>
      </div>
    </div>
  </>;
}
