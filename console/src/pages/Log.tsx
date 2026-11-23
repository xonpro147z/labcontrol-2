import { History } from "lucide-react";
import { useStore } from "../api";
import { Empty, time } from "../components/ui";

/** Nhật ký buổi học: máy vào/ra lớp, bài nộp, ứng dụng bị chặn... (chỉ giữ trong phiên làm việc này) */
export function LogPage() {
  const { log } = useStore();
  return (
    <>
      <div className="head"><div className="title"><h1>Nhật ký buổi học<small>Các sự kiện từ lúc mở LabControl</small></h1></div></div>
      <div className="page">
        <div className="card"><div className="card-b log">
          {log.length === 0 ? <Empty icon={<History size={22} />} title="Chưa có sự kiện nào" /> :
            log.map((l, i) => <div key={i}><time>{time(l.t)}</time><span className={l.kind === "warn" ? "warn" : ""}>{l.text}</span></div>)}
        </div></div>
      </div>
    </>
  );
}
