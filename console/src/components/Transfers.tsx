import { useEffect } from "react";
import { CheckCircle2, FileUp, X } from "lucide-react";
import { dismissTransfer, useStore, type Transfer } from "../api";

function size(n: number) {
  const u = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
  return `${n >= 10 || i === 0 ? Math.round(n) : n.toFixed(1)} ${u[i]}`;
}

function Item({ t }: { t: Transfer }) {
  const finished = t.done + t.failed >= t.total;
  const pct = Math.round(((t.done + t.failed) * 100) / Math.max(1, t.total));
  useEffect(() => {
    // gửi xong không lỗi thì tự ẩn sau 6 giây
    if (finished && t.failed === 0) {
      const h = setTimeout(() => dismissTransfer(t.batch), 6000);
      return () => clearTimeout(h);
    }
  }, [finished, t.failed, t.batch]);
  return (
    <div className="transfer">
      <div className="row">
        {finished && !t.failed ? <CheckCircle2 size={18} color="var(--online)" /> : <FileUp size={18} />}
        <div className="grow" style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 600, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{t.label}</div>
          <div className="small faint">{t.files} file · {size(t.bytes)} · {t.machines} máy</div>
        </div>
        <button className="btn ghost sm" onClick={() => dismissTransfer(t.batch)} aria-label="Ẩn"><X size={14} /></button>
      </div>
      <div className="prog"><i style={{ width: `${pct}%`, background: t.failed ? "var(--warn)" : undefined }} /></div>
      <div className="small">
        {finished ? (t.failed ? `Xong: ${t.done} thành công, ${t.failed} lỗi` : "Đã gửi xong – học sinh nhận ở thư mục TuGiaoVien")
          : `Đang gửi... ${t.done + t.failed}/${t.total}`}
      </div>
      {t.errors.length > 0 && <div className="errs">{t.errors.map((e, i) => <div key={i}>{e.student}: {e.name} – {e.error}</div>)}</div>}
    </div>
  );
}

/** Bảng tiến độ gửi file (góc dưới bên phải) */
export function Transfers() {
  const { transfers } = useStore();
  if (!transfers.length) return null;
  return <div className="transfers">{transfers.slice(-4).map((t) => <Item key={t.batch} t={t} />)}</div>;
}
