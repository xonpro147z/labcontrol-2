import { tr } from "../i18n";
import { useEffect, useRef, useState } from "react";
import { Lock, MessageSquare, MousePointer2, Unlock, X } from "lucide-react";
import { api, displayName, onFrame, type Student } from "../api";

/** Xem lớn và điều khiển một máy học sinh (mở ra là điều khiển ngay). */
export function Viewer({ s, onClose }: { s: Student; onClose: () => void }) {
  const [url, setUrl] = useState<string>("");
  // mở là điều khiển luôn; bấm "Thôi điều khiển" để chỉ xem
  const [ctl, setCtl] = useState(true);
  const img = useRef<HTMLImageElement>(null);
  const lastMove = useRef(0);

  useEffect(() => {
    api.send([s.id], { t: "stream_start", width: 1600, fps: 12, quality: 65 });
    const off = onFrame((id, u) => { if (id === s.id) setUrl(u); });
    return () => { off(); api.send([s.id], { t: "stream_stop" }); };
  }, [s.id]);

  // bàn phím: khi đang điều khiển, mọi phím được chuyển sang máy học sinh
  useEffect(() => {
    if (!ctl) {
      const esc = (e: KeyboardEvent) => e.key === "Escape" && onClose();
      window.addEventListener("keydown", esc);
      return () => window.removeEventListener("keydown", esc);
    }
    const kd = (e: KeyboardEvent) => { e.preventDefault(); api.input(s.id, { t: "key", key: e.key, code: e.code, down: true }); };
    const ku = (e: KeyboardEvent) => { e.preventDefault(); api.input(s.id, { t: "key", key: e.key, code: e.code, down: false }); };
    window.addEventListener("keydown", kd);
    window.addEventListener("keyup", ku);
    return () => { window.removeEventListener("keydown", kd); window.removeEventListener("keyup", ku); };
  }, [ctl, s.id, onClose]);

  const pos = (e: React.MouseEvent) => {
    const r = img.current!.getBoundingClientRect();
    return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height };
  };
  const btn = (b: number) => (b === 2 ? "right" : b === 1 ? "middle" : "left");

  return (
    <div className="viewer">
      <div className="viewer-h">
        <b style={{ fontSize: 15 }}>{displayName(s)}</b>
        <span className="small" style={{ color: "#94a3b8" }}>{s.info.user ? `👤 ${s.info.user} · ` : ""}{s.addr}</span>
        <div className="row right">
          <button className="btn sm" style={ctl ? { background: "#dc2626", color: "#fff", borderColor: "#dc2626" } : {}}
                  onClick={() => setCtl(!ctl)}>
            <MousePointer2 size={14} />{ctl ? tr("Thôi điều khiển") : tr("Điều khiển")}</button>
          <button className="btn sm" onClick={() => api.send([s.id], { t: "lock", message: tr("Máy đã bị giáo viên khóa") })}><Lock size={14} />{tr("Khóa")}</button>
          <button className="btn sm" onClick={() => api.send([s.id], { t: "unlock" })}><Unlock size={14} />{tr("Mở khóa")}</button>
          <button className="btn sm" onClick={() => {
            const t = prompt(tr("Tin nhắn gửi ") + displayName(s));
            if (t) api.send([s.id], { t: "message", text: t });
          }}><MessageSquare size={14} />{tr("Tin nhắn")}</button>
          <button className="btn sm" onClick={onClose}><X size={14} />{tr("Đóng")}</button>
        </div>
      </div>
      <div className="viewer-b" onContextMenu={(e) => ctl && e.preventDefault()}>
        {url ? <img ref={img} src={url} alt="" draggable={false} className={ctl ? "ctl" : ""}
          onMouseMove={(e) => {
            if (!ctl) return;
            const now = performance.now();
            if (now - lastMove.current < 30) return;
            lastMove.current = now;
            api.input(s.id, { t: "mouse_move", ...pos(e) });
          }}
          onMouseDown={(e) => { if (!ctl) return; e.preventDefault(); api.input(s.id, { t: "mouse_move", ...pos(e) });
                                api.input(s.id, { t: "mouse_button", button: btn(e.button), down: true }); }}
          onMouseUp={(e) => ctl && api.input(s.id, { t: "mouse_button", button: btn(e.button), down: false })}
          onWheel={(e) => ctl && api.input(s.id, { t: "wheel", dx: Math.round(e.deltaX), dy: Math.round(e.deltaY) })}
        /> : <div style={{ color: "#94a3b8" }}>{tr("Đang nhận hình...")}</div>}
      </div>
    </div>
  );
}
