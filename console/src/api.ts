/** Cầu nối tới phần Rust của Console (Tauri). */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";

export interface AgentInfo {
  id: string; name: string; os: string; user: string; version: string; room: string; screen_w: number; screen_h: number;
}
export interface Student { id: string; addr: string; info: AgentInfo; state: "connecting" | "online" | "offline" | "denied"; error: string }
export interface Status { user: string; app: string; title: string; locked: boolean; showing: boolean; viewed: boolean }
export interface Settings {
  teacher_name: string; room: string; thumb_seconds: number; thumb_width: number; manual: string[];
  show_fps: number; show_quality: number;
}

export type Ctrl =
  | { t: "lock"; message: string } | { t: "unlock" } | { t: "message"; text: string } | { t: "open_url"; url: string }
  | { t: "power"; action: "shutdown" | "restart" | "logoff" }
  | { t: "stream_start"; width: number; fps: number; quality: number } | { t: "stream_stop" };

export type InputEv =
  | { t: "mouse_move"; x: number; y: number } | { t: "mouse_button"; button: string; down: boolean }
  | { t: "wheel"; dx: number; dy: number } | { t: "key"; key: string; code: string; down: boolean } | { t: "text"; text: string };

export const api = {
  getState: () => invoke<{ teacher_code: string; settings: Settings; students: Student[]; showing: boolean }>("get_state"),
  setSettings: (settings: Settings) => invoke("set_settings", { settings }),
  addManual: (addr: string) => invoke("add_manual", { addr }),
  remove: (id: string) => invoke("remove_student", { id }),
  send: (ids: string[], msg: Ctrl) => invoke<number>("send_ctrl", { ids, msg }),
  input: (id: string, ev: InputEv) => invoke("input", { id, ev }),
  sendFiles: (ids: string[], paths: string[]) => invoke<number>("send_files", { ids, paths }),
  startShow: (ids: string[], fullscreen: boolean) => invoke("start_show", { ids, fullscreen }),
  stopShow: () => invoke("stop_show"),
};

// ------------------------------------------------------------------ kho dữ liệu dùng chung
interface Snap {
  students: Map<string, Student>;
  thumbs: Map<string, string>;
  status: Map<string, Status>;
  showing: boolean;
  teacherCode: string;
  settings: Settings | null;
  v: number;
}

let snap: Snap = { students: new Map(), thumbs: new Map(), status: new Map(), showing: false, teacherCode: "", settings: null, v: 0 };
const subs = new Set<() => void>();
function set(p: Partial<Snap>) {
  snap = { ...snap, ...p, v: snap.v + 1 };
  subs.forEach((f) => f());
}
export function useStore() {
  return useSyncExternalStore((f) => { subs.add(f); return () => { subs.delete(f); }; }, () => snap);
}
export function getStore() { return snap; }

type FrameListener = (id: string, url: string) => void;
const frameListeners = new Set<FrameListener>();
export function onFrame(f: FrameListener) { frameListeners.add(f); return () => { frameListeners.delete(f); }; }

type FileListener = (e: { student: string; ok: boolean; path: string; error: string }) => void;
const fileListeners = new Set<FileListener>();
export function onFileDone(f: FileListener) { fileListeners.add(f); return () => { fileListeners.delete(f); }; }

export const notices = new Set<(t: string) => void>();

let started = false;
export async function startStore() {
  if (started) return;
  started = true;
  await listen<Student>("student", (e) => {
    const m = new Map(snap.students); m.set(e.payload.id, e.payload); set({ students: m });
  });
  await listen<string>("student_removed", (e) => {
    const m = new Map(snap.students); m.delete(e.payload); set({ students: m });
  });
  await listen<{ id: string; jpeg: string }>("thumb", (e) => {
    const t = new Map(snap.thumbs); t.set(e.payload.id, `data:image/jpeg;base64,${e.payload.jpeg}`); set({ thumbs: t });
  });
  await listen<{ id: string; status: Status }>("status", (e) => {
    const s = new Map(snap.status); s.set(e.payload.id, e.payload.status); set({ status: s });
  });
  await listen<{ id: string; jpeg: string }>("frame", (e) => {
    const url = `data:image/jpeg;base64,${e.payload.jpeg}`;
    frameListeners.forEach((f) => f(e.payload.id, url));
  });
  await listen<{ student: string; ok: boolean; path: string; error: string }>("file_done", (e) => fileListeners.forEach((f) => f(e.payload)));
  await listen<string>("notice", (e) => notices.forEach((f) => f(e.payload)));
  await listen<boolean>("show", (e) => set({ showing: e.payload }));
  await reload();
}

export async function reload() {
  const s = await api.getState();
  set({ students: new Map(s.students.map((x) => [x.id, x])), showing: s.showing, teacherCode: s.teacher_code, settings: s.settings });
}

export function setSettingsLocal(s: Settings) { set({ settings: s }); }
