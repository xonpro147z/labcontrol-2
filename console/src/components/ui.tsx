import { tr } from "../i18n";
import { useEffect, useState, useSyncExternalStore, type ReactNode } from "react";
import { AlertTriangle, CheckCircle2, X } from "lucide-react";

// ------------------------------------------------------------------ hộp thoại
export function Modal({ title, onClose, children, footer, wide, bare }: {
  title: ReactNode; onClose: () => void; children: ReactNode; footer?: ReactNode; wide?: boolean; bare?: boolean;
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
          <button className="btn ghost sm right" onClick={onClose} aria-label={tr("Đóng")}><X size={16} /></button></div>
        {bare ? children : <div className="modal-b">{children}</div>}
        {footer && <div className="modal-f">{footer}</div>}
      </div>
    </div>
  );
}

export function Confirm({ title, message, danger, confirmText = tr("Đồng ý"), onConfirm, onClose }: {
  title: string; message: ReactNode; danger?: boolean; confirmText?: string;
  onConfirm: () => Promise<void> | void; onClose: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  return (
    <Modal title={title} onClose={onClose} footer={<>
      <button className="btn" onClick={onClose}>{tr("Hủy")}</button>
      <button className={`btn ${danger ? "solid-danger" : "primary"}`} disabled={busy} onClick={async () => {
        setBusy(true); setErr("");
        try { await onConfirm(); onClose(); } catch (e) { setErr(errText(e)); setBusy(false); }
      }}>{confirmText}</button></>}>
      <div className="muted">{message}</div>
      {err && <div className="err">{err}</div>}
    </Modal>
  );
}

/** Menu thả xuống gắn dưới một nút */
export function Menu({ anchor, onClose, children, width }: { anchor: HTMLElement; onClose: () => void; children: ReactNode; width?: number }) {
  const r = anchor.getBoundingClientRect();
  const w = width ?? 240;
  const left = Math.min(r.left, window.innerWidth - w - 12);
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [onClose]);
  return <>
    <div className="menu-bg" onMouseDown={onClose} />
    <div className="menu" style={{ top: r.bottom + 6, left, width: w }} onClick={(e) => e.stopPropagation()}>{children}</div>
  </>;
}

export function Switch({ checked, onChange, label, hint }: { checked: boolean; onChange: (v: boolean) => void; label: ReactNode; hint?: ReactNode }) {
  return <label className="switch">
    <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    <span><span>{label}</span>{hint && <div className="small faint">{hint}</div>}</span>
  </label>;
}

// ------------------------------------------------------------------ thông báo nổi
type ToastT = { id: number; text: string; kind: "ok" | "error" };
let toasts: ToastT[] = [];
const tsubs = new Set<() => void>();
export function toast(text: string, kind: "ok" | "error" = "ok") {
  const t = { id: Date.now() + Math.random(), text, kind };
  toasts = [...toasts.slice(-4), t];
  tsubs.forEach((f) => f());
  setTimeout(() => { toasts = toasts.filter((x) => x.id !== t.id); tsubs.forEach((f) => f()); }, 4500);
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

export function Spinner() { return <div className="spinner" />; }

export function Empty({ icon, title, children }: { icon: ReactNode; title: string; children?: ReactNode }) {
  return <div className="empty"><div className="ico">{icon}</div><h3>{title}</h3>{children}</div>;
}

export function time(t: number) {
  return new Date(t).toLocaleTimeString("vi-VN", { hour12: false });
}
export function dateTime(t: number) {
  return new Date(t).toLocaleString("vi-VN", { hour12: false });
}
