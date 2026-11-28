import { setLang, tr } from "./i18n";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { glassLevel, resolveMode, themeStyle } from "./theme";
import { useEffect, useState } from "react";
import { ClipboardList, Heart, History, LayoutGrid, Monitor, Moon, Settings as SettingsIcon, Sun } from "lucide-react";
import { api, infos, notices, setSettingsLocal, startStore, useStore } from "./api";
import { Spinner, toast, Toasts } from "./components/ui";
import { Transfers } from "./components/Transfers";
import { Classroom } from "./pages/Classroom";
import { LogPage } from "./pages/Log";
import { SettingsModal } from "./pages/Settings";
import { TasksPage } from "./pages/Tasks";

export type Page = "class" | "tasks" | "log";

export function App() {
  const [ready, setReady] = useState(false);
  const [page, setPage] = useState<Page>("class");
  const [settingsOpen, setSettingsOpen] = useState<string | null>(null);
  const { settings, pending, quiz } = useStore();

  useEffect(() => {
    startStore().then(() => setReady(true)).catch((e) => { toast(String(e), "error"); setReady(true); });
    const f = (t: string) => toast(t, "error");
    const g = (t: string) => toast(t);
    notices.add(f);
    infos.add(g);
    return () => { notices.delete(f); infos.delete(g); };
  }, []);

  // theo chế độ sáng/tối của hệ điều hành
  const [systemDark, setSystemDark] = useState(() => window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false);
  useEffect(() => {
    const m = window.matchMedia?.("(prefers-color-scheme: dark)");
    const f = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    m?.addEventListener("change", f);
    return () => m?.removeEventListener("change", f);
  }, []);
  // cỡ giao diện: phóng to/thu nhỏ cả cửa sổ
  const scale = settings?.font_scale || 100;
  useEffect(() => { try { getCurrentWebview().setZoom(scale / 100).catch(() => {}); } catch { /* chạy ngoài ứng dụng */ } }, [scale]);

  setLang(settings?.lang || "vi");

  if (!ready) return <div className="center"><Spinner /></div>;
  const dark = resolveMode(settings, systemDark) === "dark";
  const toggleDark = async () => {
    if (!settings) return;
    const s = { ...settings, dark: !dark, theme_mode: dark ? "light" : "dark" };
    setSettingsLocal(s);
    await api.setSettings(s).catch(() => {});
  };

  return (
    <div className={`app ${dark ? "dark" : ""} ${glassLevel(settings) === 0 ? "no-glass" : ""}`} style={themeStyle(settings, dark)}>
      <nav className="rail">
        <div className="logo" title="LabControl"><Monitor size={22} /></div>
        <button className={page === "class" ? "active" : ""} onClick={() => setPage("class")}>
          <LayoutGrid size={21} />{tr("Lớp học")}{pending.length > 0 && <span className="badge-n">{pending.length}</span>}</button>
        <button className={page === "tasks" ? "active" : ""} onClick={() => setPage("tasks")}>
          <ClipboardList size={21} />{tr("Bài & kiểm tra")}{quiz?.running && <span className="badge-n">●</span>}</button>
        <button className={page === "log" ? "active" : ""} onClick={() => setPage("log")}><History size={21} />{tr("Nhật ký")}</button>
        <button onClick={() => setSettingsOpen("look")}><SettingsIcon size={21} />{tr("Cài đặt")}</button>
        <button className="bottom" onClick={() => setSettingsOpen("author")} title={tr("Tác giả, góp ý và ủng hộ")}><Heart size={20} />{tr("Góp ý")}</button>
        <button onClick={toggleDark}>{dark ? <Sun size={20} /> : <Moon size={20} />}{dark ? tr("Nền sáng") : tr("Nền tối")}</button>
      </nav>
      <main className="main">
        {page === "class" && <Classroom openSettings={(s) => setSettingsOpen(s)} goTasks={() => setPage("tasks")} />}
        {page === "tasks" && <TasksPage />}
        {page === "log" && <LogPage />}
      </main>
      {settingsOpen && <SettingsModal initial={settingsOpen} onClose={() => setSettingsOpen(null)} />}
      <Transfers />
      <Toasts />
    </div>
  );
}
