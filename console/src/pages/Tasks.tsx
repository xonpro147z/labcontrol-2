import { tr } from "../i18n";
import { useMemo, useState } from "react";
import { BookOpen, ClipboardCheck, Download, FolderOpen, Inbox, Play, Square, Trash2 } from "lucide-react";
import { api, clearCollect, useStore, type Quiz, type QuizQuestion } from "../api";
import { Confirm, Empty, Modal, dateTime, errText, toast } from "../components/ui";

const SAMPLE = () => tr("Thủ đô của Việt Nam là?\n*Hà Nội\nHuế\nĐà Nẵng\n\nKết quả của 2 + 3 là?\n4\n*5\n6");

/** Soạn nhanh: mỗi câu cách nhau một dòng trống; dòng đầu là câu hỏi; đáp án đúng đánh dấu * ở đầu dòng. */
export function parseQuiz(src: string): { questions: QuizQuestion[]; errors: string[] } {
  const blocks = src.replace(/\r/g, "").split(/\n\s*\n/).map((b) => b.split("\n").map((l) => l.trim()).filter(Boolean)).filter((b) => b.length);
  const questions: QuizQuestion[] = [];
  const errors: string[] = [];
  blocks.forEach((b, i) => {
    const q = b[0].replace(/^((câu|question|q)\s*\d+[.:)]\s*)/i, "");
    let answer: number | null = null;
    const options = b.slice(1).map((l, j) => {
      let t = l.replace(/^[A-Ha-h][.)]\s+/, "");
      if (/^\*/.test(l) || /^\*/.test(t)) { if (answer === null) answer = j; }
      t = t.replace(/^\*\s*/, "").replace(/^[A-Ha-h][.)]\s+/, "");
      return t;
    });
    if (options.length < 2) errors.push(tr("Câu {0}: cần ít nhất 2 đáp án", i + 1));
    else if (answer === null) errors.push(tr("Câu {0}: chưa đánh dấu * cho đáp án đúng", i + 1));
    questions.push({ q, options, answer });
  });
  return { questions, errors };
}

export function TasksPage() {
  const { students } = useStore();
  const online = [...students.values()].filter((s) => s.state === "online").map((s) => s.id);
  return (
    <>
      <div className="head"><div className="title"><h1>{tr("Bài & kiểm tra")}<small>{online.length}{" "}{tr("máy trực tuyến")}</small></h1></div></div>
      <div className="page">
        <div className="two">
          <Assignments online={online} />
          <QuizCard online={online} />
        </div>
      </div>
    </>
  );
}

function Assignments({ online }: { online: string[] }) {
  const { tasks, collects, students } = useStore();
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [del, setDel] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const total = students.size;
  return (
    <div className="card">
      <div className="card-h"><BookOpen size={18} /><h2>{tr("Bài tập")}</h2></div>
      <div className="card-b">
        <div className="field"><label>{tr("Giao bài mới")}</label>
          <input className="input" placeholder={tr("Tên bài, vd: Bài 3 – Vòng lặp")} value={title} onChange={(e) => setTitle(e.target.value)} />
          <textarea className="input" rows={3} placeholder={tr("Đề bài / hướng dẫn (không bắt buộc)")} value={text} onChange={(e) => setText(e.target.value)} />
          <div className="row">
            <span className="hint">{tr("Học sinh lưu bài vào thư mục BaiLam\\<tên bài> trên màn hình nền.")}</span>
            <button className="btn primary right" disabled={!title.trim() || busy} onClick={async () => {
              if (!online.length) return toast(tr("Chưa có máy nào trực tuyến"), "error");
              setBusy(true);
              try { await api.assign(online, title, text); toast(tr("Đã giao “{0}” cho {1} máy", title.trim(), online.length)); setTitle(""); setText(""); }
              catch (e) { toast(errText(e), "error"); }
              setBusy(false);
            }}>{tr("Giao cho cả lớp")}</button>
          </div>
        </div>
        <div>
          {tasks.length === 0 ? <Empty icon={<Inbox size={22} />} title={tr("Chưa có bài tập nào")} /> : tasks.map((t) => {
            const live = collects[t.id] || {};
            const per = new Map<string, number>();
            for (const [id, n] of Object.entries(t.collected)) per.set(id, n);
            for (const [id, x] of Object.entries(live)) per.set(id, Math.max(per.get(id) ?? 0, x.files));
            const got = [...per.values()].filter((n) => n > 0).length;
            const files = [...per.values()].reduce((a, b) => a + b, 0);
            const busyN = Object.values(live).filter((x) => !x.done).length;
            return <div className="task" key={t.id}>
              <div className="grow col" style={{ gap: 4, minWidth: 0 }}>
                <b>{t.title}</b>
                <span className="small faint">{dateTime(t.created)} · {got}/{total}{" "}{tr("máy đã nộp ·")}{" "}{files} file{busyN ? tr(" · đang thu {0} máy...", busyN) : ""}</span>
                <div className="prog"><i style={{ width: `${total ? (got * 100) / total : 0}%` }} /></div>
              </div>
              <button className="btn sm primary" title={tr("Thu bài các máy đang trực tuyến")} onClick={async () => {
                try { clearCollect(t.id); const n = await api.collect(online, t.id); toast(tr("Đang thu bài từ {0} máy", n)); } catch (e) { toast(errText(e), "error"); }
              }}><Download size={14} />{tr("Thu bài")}</button>
              <button className="btn sm" title={tr("Mở thư mục bài nộp")} onClick={() => api.openTaskDir(t.id).catch((e) => toast(errText(e), "error"))}><FolderOpen size={14} /></button>
              <button className="btn sm ghost" title={tr("Xóa khỏi danh sách")} onClick={() => setDel(t.id)}><Trash2 size={14} /></button>
            </div>;
          })}
        </div>
      </div>
      {del && <Confirm title={tr("Xóa bài tập khỏi danh sách?")} message={tr("Các file đã thu trên máy giáo viên vẫn được giữ nguyên.")} confirmText={tr("Xóa")} danger
                       onClose={() => setDel(null)} onConfirm={() => api.deleteTask(del) as Promise<void>} />}
    </div>
  );
}

function QuizCard({ online }: { online: string[] }) {
  const { quiz } = useStore();
  const [title, setTitle] = useState(tr("Kiểm tra 15 phút"));
  const [minutes, setMinutes] = useState(15);
  const [src, setSrc] = useState(SAMPLE);
  const [preview, setPreview] = useState(false);
  const parsed = useMemo(() => parseQuiz(src), [src]);
  const running = quiz?.running;

  const start = async () => {
    if (parsed.errors.length) return toast(parsed.errors[0], "error");
    if (!online.length) return toast(tr("Chưa có máy nào trực tuyến"), "error");
    const q: Quiz = { id: "", title: title.trim() || tr("Trắc nghiệm"), minutes, questions: parsed.questions };
    try { const n = await api.quizStart(online, q); toast(tr("Đã phát đề cho {0} máy", n)); } catch (e) { toast(errText(e), "error"); }
  };

  return (
    <div className="card">
      <div className="card-h"><ClipboardCheck size={18} /><h2>{tr("Trắc nghiệm")}</h2>
        {quiz && <div className="row right">
          {running ? <button className="btn sm solid-danger" onClick={() => api.quizStop()}><Square size={13} />{tr("Thu bài & kết thúc")}</button>
            : <span className="chip">{tr("Đã kết thúc")}</span>}
          <button className="btn sm" onClick={() => api.openQuizCsv().catch((e) => toast(errText(e), "error"))}><FolderOpen size={14} />{tr("Kết quả (CSV)")}</button>
        </div>}
      </div>
      <div className="card-b">
        {quiz && <Results />}
        {!running && <>
          <div className="row">
            <div className="field grow"><label>{tr("Tên bài")}</label><input className="input" value={title} onChange={(e) => setTitle(e.target.value)} /></div>
            <div className="field" style={{ width: 130 }}><label>{tr("Thời gian (phút)")}</label>
              <input className="input" type="number" min={0} max={180} value={minutes} onChange={(e) => setMinutes(Math.max(0, +e.target.value || 0))} /></div>
          </div>
          <div className="field"><label>{tr("Câu hỏi")}</label>
            <textarea className="input mono" rows={12} value={src} onChange={(e) => setSrc(e.target.value)} />
            <span className="hint">{tr("Mỗi câu cách nhau một dòng trống. Dòng đầu là câu hỏi, các dòng sau là đáp án; đánh dấu")}{" "}<b>*</b>{" "}{tr("trước đáp án đúng. Thời gian 0 = không giới hạn. Học sinh không thấy đáp án.")}</span>
            {parsed.errors.length > 0 && <div className="err">{parsed.errors.join(" · ")}</div>}
          </div>
          <div className="row">
            <span className="small muted">{parsed.questions.length}{" "}{tr("câu")}</span>
            <button className="btn right" onClick={() => setPreview(true)}>{tr("Xem trước")}</button>
            <button className="btn primary" onClick={start}><Play size={14} />{tr("Phát đề cho cả lớp (")}{online.length})</button>
          </div>
        </>}
      </div>
      {preview && <Modal title={title} onClose={() => setPreview(false)} wide>
        {parsed.questions.map((q, i) => <div className="q-prev" key={i}><b>{tr("Câu")}{" "}{i + 1}. {q.q}</b>
          <ol type="A">{q.options.map((o, j) => <li key={j} className={q.answer === j ? "ok" : ""}>{o}</li>)}</ol></div>)}
      </Modal>}
    </div>
  );
}

function Results() {
  const { quiz } = useStore();
  if (!quiz) return null;
  const n = quiz.quiz.questions.length;
  const rows = Object.entries(quiz.results).sort((a, b) => a[1].name.localeCompare(b[1].name, "vi", { numeric: true }));
  const done = rows.filter(([, r]) => r.done).length;
  const avg = done ? rows.filter(([, r]) => r.done).reduce((a, [, r]) => a + (r.correct * 10) / n, 0) / done : 0;
  return <div className="col">
    <div className="row"><b>{quiz.quiz.title}</b><span className="small faint">· {n}{" "}{tr("câu ·")}{" "}{quiz.quiz.minutes ? tr("{0} phút", quiz.quiz.minutes) : tr("không giới hạn")}</span>
      <span className="chip ok right">{done}/{rows.length}{" "}{tr("đã nộp")}</span>{done > 0 && <span className="chip">{tr("Điểm TB")}{" "}{avg.toFixed(1)}</span>}</div>
    <table className="table">
      <thead><tr><th>{tr("Máy")}</th><th>{tr("Học sinh")}</th><th>{tr("Đã làm")}</th><th>{tr("Đúng")}</th><th>{tr("Điểm")}</th><th>{tr("Trạng thái")}</th></tr></thead>
      <tbody>{rows.map(([id, r]) => {
        const answered = r.answers.filter((a) => a >= 0).length;
        return <tr key={id}><td><b>{r.name}</b></td><td>{r.user || "—"}</td><td>{answered}/{n}</td><td>{r.correct}</td>
          <td className="score">{r.done ? ((r.correct * 10) / n).toFixed(1) : "—"}</td>
          <td className="small">{r.done ? <span style={{ color: "var(--online)" }}>{tr("Đã nộp")}</span> : quiz.running ? tr("Đang làm") : tr("Không nộp")}</td></tr>;
      })}</tbody>
    </table>
  </div>;
}
