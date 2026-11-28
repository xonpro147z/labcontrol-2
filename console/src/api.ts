import { tr } from "./i18n";
/** Cầu nối tới phần Rust của Console (Tauri). */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";

export interface AgentInfo {
  id: string; name: string; os: string; user: string; version: string; room: string; screen_w: number; screen_h: number;
}
export interface Status {
  user: string; app: string; title: string; locked: boolean; showing: boolean; viewed: boolean;
  exam: boolean; web_blocked: boolean; tools_blocked: boolean; helper: boolean; muted?: boolean;
}
export interface Student {
  id: string; addr: string; info: AgentInfo; state: "online" | "offline"; status: Status | null;
  seat: [number, number] | null; label: string;
}
export interface Pending { id: string; name: string; user: string; ip: string; reason: string; at: number }
export interface Settings {
  teacher_name: string; room: string; room_password: string; accept_new: boolean;
  thumb_seconds: number; thumb_width: number; show_fps: number; show_quality: number;
  web_list: string[]; app_list: string[]; run_list: string[]; dark: boolean; glass: boolean; seat_cols: number;
  lang: string; theme_mode: string; accent: string; wallpaper: string; glass_level: number; font_scale: number; radius: string;
  lock_message: string; lock_media: LockMedia;
}
export interface LockMedia { sha: string; kind: string; name: string; fps: number; frames: number; size: number; thumb: string }
export interface Policy { block_web: string[]; block_tools: boolean; block_apps: string[]; exam: boolean; mute?: boolean }
export const NO_POLICY: Policy = { block_web: [], block_tools: false, block_apps: [], exam: false, mute: false };
export interface Task { id: string; title: string; text: string; created: number; collected: Record<string, number> }
export interface QuizQuestion { q: string; options: string[]; answer?: number | null }
export interface Quiz { id: string; title: string; minutes: number; questions: QuizQuestion[] }
export interface QuizResult { answers: number[]; done: boolean; correct: number; user: string; name: string }
export interface QuizRun { quiz: Quiz; started: number; running: boolean; results: Record<string, QuizResult> }

export type Ctrl =
  | { t: "lock"; message: string; media?: string } | { t: "unlock" } | { t: "message"; text: string } | { t: "open_url"; url: string }
  | { t: "power"; action: "shutdown" | "restart" | "logoff" } | { t: "close_app"; name: string } | { t: "run_app"; cmd: string }
  | { t: "stream_start"; width: number; fps: number; quality: number } | { t: "stream_stop" };

export type InputEv =
  | { t: "mouse_move"; x: number; y: number } | { t: "mouse_button"; button: string; down: boolean }
  | { t: "wheel"; dx: number; dy: number } | { t: "key"; key: string; code: string; down: boolean } | { t: "text"; text: string };

export interface SendBatch { batch: number; files: number; machines: number; total: number; bytes: number }
export interface Transfer extends SendBatch { label: string; done: number; failed: number; errors: { student: string; name: string; error: string }[]; started: number }
export interface UpdateInfo { version: string; url: string; notes: string; newer: boolean }

interface AppState {
  teacher_code: string; settings: Settings; students: Student[]; pending: Pending[]; showing: boolean; policy: Policy; overrides: Record<string, Policy>;
  tasks: Task[]; quiz: QuizRun | null; listen_error: string; out_dir: string; addresses: string[]; version: string;
  update: UpdateInfo | null; bundled: string[];
}

export const api = {
  getState: () => invoke<AppState>("get_state"),
  setSettings: (settings: Settings) => invoke("set_settings", { settings }),
  pushPassword: () => invoke<number>("push_password"),
  remove: (id: string) => invoke("remove_student", { id }),
  dismissPending: (id: string) => invoke("dismiss_pending", { id }),
  setSeat: (id: string, seat: [number, number] | null) => invoke("set_seat", { id, seat }),
  autoSeats: (cols: number) => invoke("auto_seats", { cols }),
  setLabel: (id: string, label: string) => invoke("set_label", { id, label }),
  setLockMedia: (kind: string, name: string, fps: number, frames: string[], thumb: string) =>
    invoke<LockMedia>("set_lock_media", { kind, name, fps, frames, thumb }),
  clearLockMedia: () => invoke("clear_lock_media"),
  send: (ids: string[], msg: Ctrl) => invoke<number>("send_ctrl", { ids, msg }),
  input: (id: string, ev: InputEv) => invoke("input", { id, ev }),
  setPolicy: (policy: Policy, ids: string[] | null = null) => invoke<number>("set_policy", { policy, ids }),
  clearOverride: (ids: string[]) => invoke<number>("clear_override", { ids }),
  sendFiles: async (ids: string[], paths: string[]) => {
    const b = await invoke<SendBatch>("send_files", { ids, paths });
    const label = paths.length === 1 ? paths[0].split(/[\\/]/).pop()! : tr("{0} mục", paths.length);
    set({ transfers: [...snap.transfers, { ...b, label, done: 0, failed: 0, errors: [], started: Date.now() }] });
    return b;
  },
  startShow: (ids: string[], fullscreen: boolean) => invoke("start_show", { ids, fullscreen }),
  stopShow: () => invoke("stop_show"),
  assign: (ids: string[], title: string, text: string) => invoke<Task>("assign", { ids, title, text }),
  collect: (ids: string[], task: string) => invoke<number>("collect", { ids, task }),
  deleteTask: (task: string) => invoke("delete_task", { task }),
  openTaskDir: (task: string) => invoke("open_task_dir", { task }),
  quizStart: (ids: string[], quiz: Quiz) => invoke<number>("quiz_start", { ids, quiz }),
  quizStop: () => invoke("quiz_stop"),
  openQuizCsv: () => invoke("open_quiz_csv"),
  checkUpdate: () => invoke<UpdateInfo>("check_update"),
  pushUpdate: (ids: string[], file: string | null) => invoke<number>("push_update", { ids, file }),
  openRelease: (url: string) => invoke("open_release", { url }),
};

// ------------------------------------------------------------------ kho dữ liệu dùng chung
export interface LogItem { t: number; text: string; kind: "info" | "warn" }
interface Snap {
  students: Map<string, Student>;
  thumbs: Map<string, string>;
  pending: Pending[];
  showing: boolean;
  policy: Policy;
  overrides: Record<string, Policy>;
  tasks: Task[];
  quiz: QuizRun | null;
  teacherCode: string;
  settings: Settings | null;
  listenError: string;
  outDir: string;
  addresses: string[];
  version: string;
  update: UpdateInfo | null;
  bundled: string[];
  log: LogItem[];
  transfers: Transfer[];
  /** task -> student -> {files, done, error} */
  collects: Record<string, Record<string, { files: number; done: boolean; error?: string }>>;
  v: number;
}

let snap: Snap = {
  students: new Map(), thumbs: new Map(), pending: [], showing: false, policy: NO_POLICY, overrides: {}, tasks: [], quiz: null,
  teacherCode: "", settings: null, listenError: "", outDir: "", addresses: [], version: "", update: null, bundled: [], log: [], transfers: [], collects: {}, v: 0,
};
const subs = new Set<() => void>();
function set(p: Partial<Snap>) {
  snap = { ...snap, ...p, v: snap.v + 1 };
  subs.forEach((f) => f());
}
export function useStore() {
  return useSyncExternalStore((f) => { subs.add(f); return () => { subs.delete(f); }; }, () => snap);
}
export function getStore() { return snap; }

export function log(text: string, kind: "info" | "warn" = "info") {
  set({ log: [{ t: Date.now(), text, kind }, ...snap.log].slice(0, 500) });
}

type FrameListener = (id: string, url: string) => void;
const frameListeners = new Set<FrameListener>();
export function onFrame(f: FrameListener) { frameListeners.add(f); return () => { frameListeners.delete(f); }; }

export function dismissTransfer(batch: number) {
  set({ transfers: snap.transfers.filter((t) => t.batch !== batch) });
}

export const notices = new Set<(t: string) => void>();
export const infos = new Set<(t: string) => void>();

let started = false;
export async function startStore() {
  if (started) return;
  started = true;
  await listen<Student>("student", (e) => {
    const m = new Map(snap.students);
    const old = m.get(e.payload.id);
    if (old && old.state !== e.payload.state) log(`${e.payload.info.name} ${e.payload.state === "online" ? tr("đã vào lớp") : tr("đã thoát")}`);
    m.set(e.payload.id, e.payload); set({ students: m });
  });
  await listen<string>("student_removed", (e) => {
    const m = new Map(snap.students); m.delete(e.payload); set({ students: m });
  });
  await listen<{ id: string; jpeg: string }>("thumb", (e) => {
    const t = new Map(snap.thumbs); t.set(e.payload.id, `data:image/jpeg;base64,${e.payload.jpeg}`); set({ thumbs: t });
  });
  await listen<{ id: string; status: Status }>("status", (e) => {
    const s = snap.students.get(e.payload.id);
    if (!s) return;
    const m = new Map(snap.students); m.set(s.id, { ...s, status: e.payload.status }); set({ students: m });
  });
  await listen<{ id: string; jpeg: string }>("frame", (e) => {
    const url = `data:image/jpeg;base64,${e.payload.jpeg}`;
    frameListeners.forEach((f) => f(e.payload.id, url));
  });
  await listen<{ batch: number; student: string; name: string; ok: boolean; error: string }>("file_done", (e) => {
    const p = e.payload;
    const i = snap.transfers.findIndex((t) => t.batch === p.batch);
    if (i < 0) return;
    const t = { ...snap.transfers[i] };
    if (p.ok) t.done++;
    else {
      t.failed++;
      const who = snap.students.get(p.student)?.info.name ?? p.student;
      t.errors = [...t.errors, { student: who, name: p.name, error: p.error }].slice(-50);
      log(tr("Gửi {0} tới {1} thất bại: {2}", p.name, who, p.error), "warn");
    }
    const list = [...snap.transfers]; list[i] = t;
    if (t.done + t.failed >= t.total) log(tr("Đã gửi “{0}” tới {1} máy: {2}/{3} file thành công", t.label, t.machines, t.done, t.total));
    set({ transfers: list });
  });
  await listen<string>("notice", (e) => { log(e.payload, "warn"); notices.forEach((f) => f(e.payload)); });
  await listen<boolean>("show", (e) => set({ showing: e.payload }));
  await listen<string>("info", (e) => { log(e.payload); infos.forEach((f) => f(e.payload)); });
  await listen<UpdateInfo>("update", (e) => set({ update: e.payload }));
  await listen<Pending[]>("pending", (e) => set({ pending: e.payload }));
  await listen<{ policy: Policy; overrides: Record<string, Policy> }>("policy", (e) => set({ policy: e.payload.policy, overrides: e.payload.overrides }));
  await listen<Task[]>("tasks", (e) => set({ tasks: e.payload }));
  await listen<QuizRun>("quiz", (e) => set({ quiz: e.payload }));
  await listen<{ task: string; student: string; name: string; done: boolean; files: number; error?: string }>("collect", (e) => {
    const p = e.payload;
    const c = { ...snap.collects, [p.task]: { ...(snap.collects[p.task] || {}), [p.student]: { files: p.files, done: p.done, error: p.error } } };
    if (p.done) log(p.error ? tr("{0}: không nộp được bài ({1})", p.name, p.error) : tr("{0} đã nộp {1} file", p.name, p.files), p.error ? "warn" : "info");
    set({ collects: c });
  });
  await reload();
}

export async function reload() {
  const s = await api.getState();
  set({
    students: new Map(s.students.map((x) => [x.id, x])), pending: s.pending, showing: s.showing, policy: s.policy, overrides: s.overrides ?? {},
    tasks: s.tasks, quiz: s.quiz, teacherCode: s.teacher_code, settings: s.settings, listenError: s.listen_error,
    outDir: s.out_dir, addresses: s.addresses, version: s.version, update: s.update, bundled: s.bundled,
  });
}

export function setSettingsLocal(s: Settings) { set({ settings: s }); }

export function clearCollect(task: string) {
  const c = { ...snap.collects };
  delete c[task];
  set({ collects: c });
}

export function displayName(s: Student) {
  return s.label || s.info.name;
}
