import { useMemo, useState } from "react";
import { BookOpen, ClipboardCheck, Download, FolderOpen, Inbox, Play, Square, Trash2 } from "lucide-react";
import { api, clearCollect, useStore, type Quiz, type QuizQuestion } from "../api";
import { Confirm, Empty, Modal, dateTime, errText, toast } from "../components/ui";

const SAMPLE = `Thủ đô của Việt Nam là?
*Hà Nội
Huế
Đà Nẵng

Kết quả của 2 + 3 là?
4
*5
6`;

/** Soạn nhanh: mỗi câu cách nhau một dòng trống; dòng đầu là câu hỏi; đáp án đúng đánh dấu * ở đầu dòng. */
export function parseQuiz(src: string): { questions: QuizQuestion[]; errors: string[] } {
  const blocks = src.replace(/\r/g, "").split(/\n\s*\n/).map((b) => b.split("\n").map((l) => l.trim()).filter(Boolean)).filter((b) => b.length);
  const questions: QuizQuestion[] = [];
  const errors: string[] = [];
  blocks.forEach((b, i) => {
    const q = b[0].replace(/^(câu\s*\d+[.:)]\s*)/i, "");
    let answer: number | null = null;
    const options = b.slice(1).map((l, j) => {
      let t = l.replace(/^[A-Ha-h][.)]\s+/, "");
      if (/^\*/.test(l) || /^\*/.test(t)) { if (answer === null) answer = j; }
      t = t.replace(/^\*\s*/, "").replace(/^[A-Ha-h][.)]\s+/, "");
      return t;
    });
    if (options.length < 2) errors.push(`Câu ${i + 1}: cần ít nhất 2 đáp án`);
    else if (answer === null) errors.push(`Câu ${i + 1}: chưa đánh dấu * cho đáp án đúng`);
    questions.push({ q, options, answer });
  });
  return { questions, errors };
}

export function TasksPage() {
  const { students } = useStore();
  const online = [...students.values()].filter((s) => s.state === "online").map((s) => s.id);
  return (
    <>
      <div className="head"><div className="title"><h1>Bài & kiểm tra<small>{online.length} máy trực tuyến</small></h1></div></div>
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
      <div className="card-h"><BookOpen size={18} /><h2>Bài tập</h2></div>
      <div className="card-b">
        <div className="field"><label>Giao bài mới</label>
          <input className="input" placeholder="Tên bài, vd: Bài 3 – Vòng lặp" value={title} onChange={(e) => setTitle(e.target.value)} />
          <textarea className="input" rows={3} placeholder="Đề bài / hướng dẫn (không bắt buộc)" value={text} onChange={(e) => setText(e.target.value)} />
          <div className="row">
            <span className="hint">Học sinh lưu bài vào thư mục BaiLam\&lt;tên bài&gt; trên màn hình nền.</span>
            <button className="btn primary right" disabled={!title.trim() || busy} onClick={async () => {
              if (!online.length) return toast("Chưa có máy nào trực tuyến", "error");
              setBusy(true);
              try { await api.assign(online, title, text); toast(`Đã giao “${title.trim()}” cho ${online.length} máy`); setTitle(""); setText(""); }
              catch (e) { toast(errText(e), "error"); }
              setBusy(false);
            }}>Giao cho cả lớp</button>
          </div>
        </div>
        <div>
          {tasks.length === 0 ? <Empty icon={<Inbox size={22} />} title="Chưa có bài tập nào" /> : tasks.map((t) => {
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
                <span className="small faint">{dateTime(t.created)} · {got}/{total} máy đã nộp · {files} file{busyN ? ` · đang thu ${busyN} máy...` : ""}</span>
                <div className="prog"><i style={{ width: `${total ? (got * 100) / total : 0}%` }} /></div>
              </div>
              <button className="btn sm primary" title="Thu bài các máy đang trực tuyến" onClick={async () => {
                try { clearCollect(t.id); const n = await api.collect(online, t.id); toast(`Đang thu bài từ ${n} máy`); } catch (e) { toast(errText(e), "error"); }
              }}><Download size={14} />Thu bài</button>
              <button className="btn sm" title="Mở thư mục bài nộp" onClick={() => api.openTaskDir(t.id).catch((e) => toast(errText(e), "error"))}><FolderOpen size={14} /></button>
              <button className="btn sm ghost" title="Xóa khỏi danh sách" onClick={() => setDel(t.id)}><Trash2 size={14} /></button>
            </div>;
          })}
        </div>
      </div>
      {del && <Confirm title="Xóa bài tập khỏi danh sách?" message="Các file đã thu trên máy giáo viên vẫn được giữ nguyên." confirmText="Xóa" danger
                       onClose={() => setDel(null)} onConfirm={() => api.deleteTask(del) as Promise<void>} />}
    </div>
  );
}

function QuizCard({ online }: { online: string[] }) {
  const { quiz } = useStore();
  const [title, setTitle] = useState("Kiểm tra 15 phút");
  const [minutes, setMinutes] = useState(15);
  const [src, setSrc] = useState(SAMPLE);
  const [preview, setPreview] = useState(false);
  const parsed = useMemo(() => parseQuiz(src), [src]);
  const running = quiz?.running;

  const start = async () => {
    if (parsed.errors.length) return toast(parsed.errors[0], "error");
    if (!online.length) return toast("Chưa có máy nào trực tuyến", "error");
    const q: Quiz = { id: "", title: title.trim() || "Trắc nghiệm", minutes, questions: parsed.questions };
    try { const n = await api.quizStart(online, q); toast(`Đã phát đề cho ${n} máy`); } catch (e) { toast(errText(e), "error"); }
  };

  return (
    <div className="card">
      <div className="card-h"><ClipboardCheck size={18} /><h2>Trắc nghiệm</h2>
        {quiz && <div className="row right">
          {running ? <button className="btn sm solid-danger" onClick={() => api.quizStop()}><Square size={13} />Thu bài & kết thúc</button>
            : <span className="chip">Đã kết thúc</span>}
          <button className="btn sm" onClick={() => api.openQuizCsv().catch((e) => toast(errText(e), "error"))}><FolderOpen size={14} />Kết quả (CSV)</button>
        </div>}
      </div>
      <div className="card-b">
        {quiz && <Results />}
        {!running && <>
          <div className="row">
            <div className="field grow"><label>Tên bài</label><input className="input" value={title} onChange={(e) => setTitle(e.target.value)} /></div>
            <div className="field" style={{ width: 130 }}><label>Thời gian (phút)</label>
              <input className="input" type="number" min={0} max={180} value={minutes} onChange={(e) => setMinutes(Math.max(0, +e.target.value || 0))} /></div>
          </div>
          <div className="field"><label>Câu hỏi</label>
            <textarea className="input mono" rows={12} value={src} onChange={(e) => setSrc(e.target.value)} />
            <span className="hint">Mỗi câu cách nhau một dòng trống. Dòng đầu là câu hỏi, các dòng sau là đáp án; đánh dấu <b>*</b> trước đáp án đúng.
              Thời gian 0 = không giới hạn. Học sinh không thấy đáp án.</span>
            {parsed.errors.length > 0 && <div className="err">{parsed.errors.join(" · ")}</div>}
          </div>
          <div className="row">
            <span className="small muted">{parsed.questions.length} câu</span>
            <button className="btn right" onClick={() => setPreview(true)}>Xem trước</button>
            <button className="btn primary" onClick={start}><Play size={14} />Phát đề cho cả lớp ({online.length})</button>
          </div>
        </>}
      </div>
      {preview && <Modal title={title} onClose={() => setPreview(false)} wide>
        {parsed.questions.map((q, i) => <div className="q-prev" key={i}><b>Câu {i + 1}. {q.q}</b>
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
    <div className="row"><b>{quiz.quiz.title}</b><span className="small faint">· {n} câu · {quiz.quiz.minutes ? `${quiz.quiz.minutes} phút` : "không giới hạn"}</span>
      <span className="chip ok right">{done}/{rows.length} đã nộp</span>{done > 0 && <span className="chip">Điểm TB {avg.toFixed(1)}</span>}</div>
    <table className="table">
      <thead><tr><th>Máy</th><th>Học sinh</th><th>Đã làm</th><th>Đúng</th><th>Điểm</th><th>Trạng thái</th></tr></thead>
      <tbody>{rows.map(([id, r]) => {
        const answered = r.answers.filter((a) => a >= 0).length;
        return <tr key={id}><td><b>{r.name}</b></td><td>{r.user || "—"}</td><td>{answered}/{n}</td><td>{r.correct}</td>
          <td className="score">{r.done ? ((r.correct * 10) / n).toFixed(1) : "—"}</td>
          <td className="small">{r.done ? <span style={{ color: "var(--online)" }}>Đã nộp</span> : quiz.running ? "Đang làm" : "Không nộp"}</td></tr>;
      })}</tbody>
    </table>
  </div>;
}
