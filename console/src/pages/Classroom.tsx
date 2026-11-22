import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  CheckSquare, Globe, Lock, MessageSquare, MonitorPlay, MonitorStop, Paperclip, Power, Unlock, Users, ZoomIn,
} from "lucide-react";
import { api, onFileDone, useStore, type Student } from "../api";
import { Topbar } from "../App";
import { Confirm, Empty, errText, Modal, toast } from "../components/ui";
import { Viewer } from "./Viewer";

function natural(a: string, b: string) {
  return a.localeCompare(b, "vi", { numeric: true, sensitivity: "base" });
}

export function Classroom({ goSettings }: { goSettings: () => void }) {
  const { students, thumbs, status, showing } = useStore();
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [size, setSize] = useState(() => Number(localStorage.getItem("lc.size") || 260));
  const [view, setView] = useState<string | null>(null);
  const [dlg, setDlg] = useState<null | "lock" | "message" | "url" | "show" | "power">(null);
  const [powerMenu, setPowerMenu] = useState(false);
  const [power, setPower] = useState<"shutdown" | "restart" | "logoff">("shutdown");

  const list = useMemo(() => [...students.values()].sort((a, b) =>
    natural(a.info.name || a.addr, b.info.name || b.addr)), [students]);
  const online = list.filter((s) => s.state === "online");
  const picked = online.filter((s) => sel.has(s.id));
  const targets = picked.length ? picked : online;
  const scope = picked.length ? `${picked.length} máy đã chọn` : `cả lớp (${online.length} máy)`;
  const ids = targets.map((s) => s.id);

  useEffect(() => onFileDone((e) => {
    if (!e.ok) toast(`Gửi file lỗi: ${e.error}`, "error");
  }), []);

  const toggle = (id: string) => setSel((s) => { const n = new Set(s); if (n.has(id)) n.delete(id); else n.add(id); return n; });
  const need = () => { if (!ids.length) { toast("Chưa có máy học sinh nào kết nối", "error"); return false; } return true; };
  const send = async (msg: Parameters<typeof api.send>[1], ok: string) => {
    if (!need()) return;
    try { const n = await api.send(ids, msg); toast(`${ok} (${n} máy)`); } catch (e) { toast(errText(e), "error"); }
  };

  async function sendFiles() {
    if (!need()) return;
    const p = await open({ multiple: true, title: "Chọn file gửi cho học sinh" });
    const paths = Array.isArray(p) ? p : p ? [p] : [];
    if (!paths.length) return;
    const n = await api.sendFiles(ids, paths as string[]);
    toast(`Đang gửi ${paths.length} file tới ${targets.length} máy (${n} lượt). File nằm ở Desktop\\TuGiaoVien của học sinh.`);
  }

  const cols = `repeat(auto-fill, minmax(${size}px, 1fr))`;

  return <>
    <Topbar title="Màn hình cả lớp" crumb={`${online.length}/${list.length} máy đang kết nối · áp dụng cho ${scope}`}>
      <div className="row small muted"><ZoomIn size={15} />
        <input type="range" min={180} max={520} value={size} onChange={(e) => { setSize(+e.target.value); localStorage.setItem("lc.size", e.target.value); }} /></div>
    </Topbar>
    <div className="page">
      <div className="toolbar">
        {showing
          ? <button className="btn primary" style={{ background: "#dc2626", borderColor: "#dc2626" }} onClick={() => api.stopShow()}>
              <MonitorStop size={16} />Dừng trình chiếu</button>
          : <button className="btn primary" onClick={() => need() && setDlg("show")}><MonitorPlay size={16} />Trình chiếu</button>}
        <button className="btn" onClick={() => need() && setDlg("lock")}><Lock size={15} />Khóa</button>
        <button className="btn" onClick={() => send({ t: "unlock" }, "Đã mở khóa")}><Unlock size={15} />Mở khóa</button>
        <button className="btn" onClick={() => need() && setDlg("message")}><MessageSquare size={15} />Tin nhắn</button>
        <button className="btn" onClick={() => need() && setDlg("url")}><Globe size={15} />Mở web</button>
        <button className="btn" onClick={sendFiles}><Paperclip size={15} />Gửi file</button>
        <div style={{ position: "relative" }}>
          <button className="btn" onClick={() => setPowerMenu(!powerMenu)}><Power size={15} />Nguồn</button>
          {powerMenu && <div className="menu" onMouseLeave={() => setPowerMenu(false)}>
            {([["shutdown", "Tắt máy"], ["restart", "Khởi động lại"], ["logoff", "Đăng xuất"]] as const).map(([k, l]) => (
              <button key={k} onClick={() => { setPowerMenu(false); setPower(k); if (need()) setDlg("power"); }}>{l}</button>))}
          </div>}
        </div>
        <button className="btn ghost right" onClick={() => setSel(sel.size === online.length ? new Set() : new Set(online.map((s) => s.id)))}>
          <CheckSquare size={15} />{sel.size && sel.size === online.length ? "Bỏ chọn" : "Chọn tất cả"}</button>
      </div>
      <div className="small faint">Bấm vào màn hình để chọn máy, bấm đúp để xem lớn và điều khiển. Không chọn máy nào = áp dụng cho cả lớp.</div>

      {list.length === 0 ? <div className="card"><Empty icon={<Users size={22} />} title="Chưa thấy máy học sinh nào">
        <div className="muted small" style={{ maxWidth: 520 }}>Cài LabControl Agent lên máy học sinh bằng <b>mã giáo viên</b> của bạn
          (trong Cài đặt). Máy học sinh cùng mạng sẽ tự hiện ở đây sau vài giây.</div>
        <button className="btn primary" onClick={goSettings}>Xem mã giáo viên & cách cài</button>
      </Empty></div> :
        <div className="tiles" style={{ gridTemplateColumns: cols }}>
          {list.map((s) => <Tile key={s.id} s={s} thumb={thumbs.get(s.id)} st={status.get(s.id)} sel={sel.has(s.id)}
                                 onClick={() => s.state === "online" && toggle(s.id)}
                                 onOpen={() => s.state === "online" && setView(s.id)} />)}
        </div>}
    </div>

    {view && students.get(view) && <Viewer s={students.get(view)!} onClose={() => setView(null)} />}

    {dlg === "show" && <ShowDialog scope={scope} onClose={() => setDlg(null)} onStart={async (fs) => {
      try { await api.startShow(ids, fs); toast(`Đang trình chiếu cho ${scope}`); } catch (e) { toast(errText(e), "error"); }
    }} />}
    {dlg === "lock" && <TextDialog title={`Khóa ${scope}`} label="Lời nhắn trên màn hình khóa" initial="Hãy chú ý lên giáo viên"
      ok="Khóa máy" onClose={() => setDlg(null)} onOk={(t) => send({ t: "lock", message: t }, "Đã khóa")} />}
    {dlg === "message" && <TextDialog title={`Gửi tin nhắn cho ${scope}`} label="Nội dung" multiline ok="Gửi"
      onClose={() => setDlg(null)} onOk={(t) => send({ t: "message", text: t }, "Đã gửi tin nhắn")} />}
    {dlg === "url" && <TextDialog title={`Mở trang web trên ${scope}`} label="Địa chỉ" initial="https://scratch.mit.edu" ok="Mở"
      onClose={() => setDlg(null)} onOk={(t) => send({ t: "open_url", url: /^https?:\/\//i.test(t) ? t : `https://${t}` }, "Đã mở trang web")} />}
    {dlg === "power" && <Confirm title="Xác nhận" danger confirmText="Thực hiện" onClose={() => setDlg(null)}
      message={`${{ shutdown: "Tắt", restart: "Khởi động lại", logoff: "Đăng xuất" }[power]} ${scope}?`}
      onConfirm={() => send({ t: "power", action: power }, "Đã gửi lệnh")} />}
  </>;
}

function Tile({ s, thumb, st, sel, onClick, onOpen }: {
  s: Student; thumb?: string; st?: { user: string; app: string; title: string; locked: boolean; showing: boolean; viewed: boolean };
  sel: boolean; onClick: () => void; onOpen: () => void;
}) {
  const on = s.state === "online";
  const color = on ? "#16a34a" : s.state === "connecting" ? "#2563eb" : s.state === "denied" ? "#dc2626" : "#64748b";
  const label = { online: "", connecting: "Đang kết nối...", offline: "Mất kết nối", denied: "Máy không nhận mã giáo viên này" }[s.state];
  return (
    <div className={`tile ${sel ? "sel" : ""} ${on ? "" : "off"}`} onClick={onClick} onDoubleClick={onOpen}>
      <div className="shot">
        {on && thumb ? <img src={thumb} alt="" draggable={false} /> : <span>{label || "Đang tải màn hình..."}</span>}
        <div className="flags">
          {st?.locked && <span>🔒 Khóa</span>}
          {st?.showing && <span>📺 Xem chiếu</span>}
        </div>
      </div>
      <div className="foot">
        <div className="name"><i className="dot" style={{ background: color }} />{s.info.name || s.addr}</div>
        <div className="sub" title={st?.title}>
          {on ? <>{st?.user || s.info.user ? `👤 ${st?.user || s.info.user}` : ""}{st?.app ? ` · ${st.app}` : ""}{st?.title ? `: ${st.title}` : ""}</>
            : s.error || s.addr}
        </div>
      </div>
    </div>
  );
}

function TextDialog({ title, label, initial = "", ok, multiline, onClose, onOk }: {
  title: string; label: string; initial?: string; ok: string; multiline?: boolean; onClose: () => void; onOk: (t: string) => void;
}) {
  const [t, setT] = useState(initial);
  const submit = () => { if (t.trim()) { onOk(t.trim()); onClose(); } };
  return <Modal title={title} onClose={onClose} footer={<>
    <button className="btn" onClick={onClose}>Hủy</button>
    <button className="btn primary" onClick={submit}>{ok}</button></>}>
    <div className="field"><label>{label}</label>
      {multiline
        ? <textarea className="input" rows={4} value={t} onChange={(e) => setT(e.target.value)} autoFocus />
        : <input className="input" value={t} onChange={(e) => setT(e.target.value)} autoFocus
                 onKeyDown={(e) => e.key === "Enter" && submit()} />}</div>
  </Modal>;
}

function ShowDialog({ scope, onClose, onStart }: { scope: string; onClose: () => void; onStart: (fullscreen: boolean) => void }) {
  const [fs, setFs] = useState(true);
  return <Modal title="Trình chiếu màn hình giáo viên" onClose={onClose} footer={<>
    <button className="btn" onClick={onClose}>Hủy</button>
    <button className="btn primary" onClick={() => { onStart(fs); onClose(); }}><MonitorPlay size={15} />Bắt đầu</button></>}>
    <div className="muted">Màn hình của bạn sẽ hiện trên {scope}.</div>
    <label className="check"><input type="checkbox" checked={fs} onChange={(e) => setFs(e.target.checked)} />
      Toàn màn hình (học sinh không tắt được cửa sổ trình chiếu)</label>
    <div className="small faint">Mẹo: thu nhỏ cửa sổ LabControl trước khi giảng. Máy học sinh vào lớp muộn sẽ tự nhận trình chiếu.</div>
  </Modal>;
}
