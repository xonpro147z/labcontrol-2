import { useEffect, useState } from "react";
import { ClipboardList, History, LayoutGrid, Monitor, Moon, Settings as SettingsIcon, Sun } from "lucide-react";
import { api, notices, setSettingsLocal, startStore, useStore } from "./api";
import { Spinner, toast, Toasts } from "./components/ui";
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
    notices.add(f);
    return () => { notices.delete(f); };
  }, []);

  if (!ready) return <div className="center"><Spinner /></div>;
  const dark = !!settings?.dark;
  const toggleDark = async () => {
    if (!settings) return;
    const s = { ...settings, dark: !dark };
    setSettingsLocal(s);
    await api.setSettings(s).catch(() => {});
  };

  return (
    <div className={`app ${dark ? "dark" : ""}`}>
      <nav className="rail">
        <div className="logo" title="LabControl"><Monitor size={22} /></div>
        <button className={page === "class" ? "active" : ""} onClick={() => setPage("class")}>
          <LayoutGrid size={21} />Lớp học{pending.length > 0 && <span className="badge-n">{pending.length}</span>}</button>
        <button className={page === "tasks" ? "active" : ""} onClick={() => setPage("tasks")}>
          <ClipboardList size={21} />Bài & kiểm tra{quiz?.running && <span className="badge-n">●</span>}</button>
        <button className={page === "log" ? "active" : ""} onClick={() => setPage("log")}><History size={21} />Nhật ký</button>
        <button onClick={() => setSettingsOpen("room")}><SettingsIcon size={21} />Cài đặt</button>
        <button className="bottom" onClick={toggleDark}>{dark ? <Sun size={20} /> : <Moon size={20} />}{dark ? "Nền sáng" : "Nền tối"}</button>
      </nav>
      <main className="main">
        {page === "class" && <Classroom openSettings={(s) => setSettingsOpen(s)} goTasks={() => setPage("tasks")} />}
        {page === "tasks" && <TasksPage />}
        {page === "log" && <LogPage />}
      </main>
      {settingsOpen && <SettingsModal initial={settingsOpen} onClose={() => setSettingsOpen(null)} />}
      <Toasts />
    </div>
  );
}
