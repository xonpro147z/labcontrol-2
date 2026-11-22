import { useEffect, useState, useSyncExternalStore, type ReactNode } from "react";
import { AlertTriangle, CheckCircle2, X } from "lucide-react";
type Status = "online" | "offline" | "warning" | "error" | "connecting";

export const STATUS: Record<Status, { label: string; color: string }> = {
  online: { label: "Online", color: "#16A34A" },
  warning: { label: "Cảnh báo", color: "#F59E0B" },
  error: { label: "Lỗi", color: "#DC2626" },
  connecting: { label: "Đang kết nối", color: "#2563EB" },
  offline: { label: "Offline", color: "#64748B" },
};
export const STATUS_ORDER: Status[] = ["online", "warning", "error", "connecting", "offline"];

export function StatusBadge({ s }: { s: Status }) {
  return <span className={`badge st-${s}`}><i className="dot" />{STATUS[s].label}</span>;
}

// ------------------------------------------------------------------ định dạng
export function bytes(n: number | null | undefined) {
  if (!n) return "—";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0, v = n;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v >= 10 || i === 0 ? Math.round(v) : v.toFixed(1)} ${u[i]}`;
}

export function pct(n: number | null | undefined) {
  return n == null ? "—" : `${Math.round(n)}%`;
}

export function ago(iso: string | null | undefined) {
  if (!iso) return "chưa bao giờ";
  const s = Math.max(0, (Date.now() - new Date(iso).getTime()) / 1000);
  if (s < 10) return "vừa xong";
  if (s < 60) return `${Math.floor(s)} giây trước`;
  if (s < 3600) return `${Math.floor(s / 60)} phút trước`;
  if (s < 86400) return `${Math.floor(s / 3600)} giờ trước`;
  return `${Math.floor(s / 86400)} ngày trước`;
}

export function dt(iso: string | null | undefined) {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("vi-VN", { hour12: false });
}

export function duration(sec: number | null | undefined) {
  if (sec == null) return "—";
  const d = Math.floor(sec / 86400), h = Math.floor((sec % 86400) / 3600), m = Math.floor((sec % 3600) / 60);
  return d ? `${d} ngày ${h} giờ` : h ? `${h} giờ ${m} phút` : `${m} phút`;
}

/** Làm mới các nhãn "x phút trước" mỗi 15 giây. */
export function useTicker(ms = 15000) {
  const [, set] = useState(0);
  useEffect(() => { const t = setInterval(() => set((x) => x + 1), ms); return () => clearInterval(t); }, [ms]);
}

// ------------------------------------------------------------------ hộp thoại
export function Modal({ title, onClose, children, footer, wide }: {
  title: ReactNode; onClose: () => void; children: ReactNode; footer?: ReactNode; wide?: boolean;
}) {
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [onClose]);
  return (
    <div className="modal-bg" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={`modal ${wide ? "wide" : ""}`} role="dialog" aria-modal>
        <div className="modal-h"><h2>{title}</h2>
          <button className="btn ghost sm right" onClick={onClose} aria-label="Đóng"><X size={16} /></button></div>
        <div className="modal-b">{children}</div>
        {footer && <div className="modal-f">{footer}</div>}
      </div>
    </div>
  );
}

export function Confirm({ title, message, danger, confirmText = "Đồng ý", onConfirm, onClose }: {
  title: string; message: ReactNode; danger?: boolean; confirmText?: string;
  onConfirm: () => Promise<void> | void; onClose: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  return (
    <Modal title={title} onClose={onClose} footer={<>
      <button className="btn" onClick={onClose}>Hủy</button>
      <button className={`btn ${danger ? "danger" : "primary"}`} disabled={busy} onClick={async () => {
        setBusy(true); setErr("");
        try { await onConfirm(); onClose(); } catch (e) { setErr(e instanceof Error ? e.message : String(e)); setBusy(false); }
      }}>{confirmText}</button></>}>
      <div className="muted">{message}</div>
      {err && <div className="err">{err}</div>}
    </Modal>
  );
}

// ------------------------------------------------------------------ thông báo nổi
type ToastT = { id: number; text: string; kind: "ok" | "error" };
let toasts: ToastT[] = [];
const tsubs = new Set<() => void>();
export function toast(text: string, kind: "ok" | "error" = "ok") {
  const t = { id: Date.now() + Math.random(), text, kind };
  toasts = [...toasts, t];
  tsubs.forEach((f) => f());
  setTimeout(() => { toasts = toasts.filter((x) => x.id !== t.id); tsubs.forEach((f) => f()); }, 4000);
}
export function Toasts() {
  const list = useSyncExternalStore((f) => { tsubs.add(f); return () => { tsubs.delete(f); }; }, () => toasts);
  return <div className="toasts">{list.map((t) => (
    <div key={t.id} className={`toast ${t.kind}`}>
      {t.kind === "ok" ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}{t.text}</div>))}</div>;
}

export function errText(e: unknown) {
  return e instanceof Error ? e.message : String(e);
}

// ------------------------------------------------------------------ biểu đồ nhỏ (SVG, không thêm thư viện)
export function Donut({ parts, size = 132, label, sub }: {
  parts: { value: number; color: string }[]; size?: number; label: ReactNode; sub?: string;
}) {
  const total = parts.reduce((a, p) => a + p.value, 0);
  const r = size / 2 - 10, c = 2 * Math.PI * r;
  let off = 0;
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img">
      <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="#eef0f5" strokeWidth={14} />
      {total > 0 && parts.filter((p) => p.value > 0).map((p, i) => {
        const len = (p.value / total) * c;
        const el = <circle key={i} cx={size / 2} cy={size / 2} r={r} fill="none" stroke={p.color} strokeWidth={14}
                           strokeDasharray={`${len} ${c - len}`} strokeDashoffset={-off}
                           transform={`rotate(-90 ${size / 2} ${size / 2})`} />;
        off += len;
        return el;
      })}
      <text x="50%" y="48%" textAnchor="middle" fontSize="26" fontWeight="700" fill="#0f172a">{label}</text>
      {sub && <text x="50%" y="64%" textAnchor="middle" fontSize="11.5" fill="#64748b">{sub}</text>}
    </svg>
  );
}

export function LineChart({ series, height = 150 }: {
  series: { name: string; color: string; points: { t: number; v: number }[] }[]; height?: number;
}) {
  const all = series.flatMap((s) => s.points);
  if (all.length < 2) return <div className="empty small">Chưa đủ dữ liệu để vẽ biểu đồ</div>;
  const W = 560, H = height, pad = 26;
  const t0 = Math.min(...all.map((p) => p.t)), t1 = Math.max(...all.map((p) => p.t));
  const x = (t: number) => pad + ((t - t0) / Math.max(1, t1 - t0)) * (W - pad - 6);
  const y = (v: number) => 8 + (1 - v / 100) * (H - 30);
  const hm = (t: number) => new Date(t).toLocaleTimeString("vi-VN", { hour: "2-digit", minute: "2-digit" });
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img">
      {[0, 50, 100].map((g) => <g key={g}>
        <line x1={pad} x2={W - 6} y1={y(g)} y2={y(g)} stroke="#eef0f5" />
        <text x={2} y={y(g) + 4} fontSize="10" fill="#94a3b8">{g}%</text></g>)}
      {series.map((s) => (
        <polyline key={s.name} fill="none" stroke={s.color} strokeWidth={1.8} strokeLinejoin="round"
                  points={s.points.map((p) => `${x(p.t)},${y(p.v)}`).join(" ")} />))}
      <text x={pad} y={H - 4} fontSize="10" fill="#94a3b8">{hm(t0)}</text>
      <text x={W - 6} y={H - 4} fontSize="10" fill="#94a3b8" textAnchor="end">{hm(t1)}</text>
    </svg>
  );
}

export function UsageBar({ label, v }: { label: string; v: number | null }) {
  const cls = v == null ? "" : v >= 90 ? "crit" : v >= 75 ? "hi" : "";
  return <>
    <span>{label}</span>
    <div className="bar"><i className={cls} style={{ width: `${v ?? 0}%` }} /></div>
    <span style={{ textAlign: "right" }}>{pct(v)}</span>
  </>;
}

export function Spinner() { return <div className="spinner" />; }

export function Empty({ icon, title, children }: { icon: ReactNode; title: string; children?: ReactNode }) {
  return <div className="empty"><div className="ico">{icon}</div><h3>{title}</h3>{children}</div>;
}
