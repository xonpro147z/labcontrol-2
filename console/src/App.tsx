import { useEffect, useState, type ReactNode } from "react";
import { Monitor, Settings as SettingsIcon, Users } from "lucide-react";
import { notices, startStore, useStore } from "./api";
import { SourceLink } from "./components/About";
import { Spinner, toast, Toasts } from "./components/ui";
import { Classroom } from "./pages/Classroom";
import { SettingsPage } from "./pages/Settings";

export function App() {
  const [ready, setReady] = useState(false);
  const [page, setPage] = useState<"class" | "settings">("class");
  const { students, settings } = useStore();

  useEffect(() => {
    startStore().then(() => setReady(true)).catch((e) => { toast(String(e), "error"); setReady(true); });
    const f = (t: string) => toast(t, "error");
    notices.add(f);
    return () => { notices.delete(f); };
  }, []);

  if (!ready) return <div className="center"><Spinner /></div>;
  const online = [...students.values()].filter((s) => s.state === "online").length;

  const nav = (k: "class" | "settings", label: string, icon: ReactNode, extra?: ReactNode) => (
    <button className={`nav ${page === k ? "active" : ""}`} onClick={() => setPage(k)}>{icon}{label}{extra}</button>
  );

  return (
    <div className="app">
      <aside className="side">
        <div className="brand">
          <div className="logo"><Monitor size={18} color="#fff" /></div>
          <div><b>LabControl</b><span>{settings?.teacher_name || "Máy giáo viên"}</span></div>
        </div>
        <div className="nav-group">Lớp học</div>
        {nav("class", "Màn hình cả lớp", <Users size={17} />, <span className="tag">{online}/{students.size}</span>)}
        <div className="nav-group">Hệ thống</div>
        {nav("settings", "Cài đặt", <SettingsIcon size={17} />)}
        <div className="side-foot">
          <div className="small" style={{ color: "#94a3b8", padding: "0 4px 6px" }}>
            {settings?.room ? <>Phòng: <b style={{ color: "#fff" }}>{settings.room}</b></> : "Mọi máy trong mạng"}
          </div>
          <SourceLink dark />
        </div>
      </aside>
      <main className="main">
        {page === "class" ? <Classroom goSettings={() => setPage("settings")} /> : <SettingsPage />}
      </main>
      <Toasts />
    </div>
  );
}

export function Topbar({ title, crumb, children }: { title: ReactNode; crumb?: ReactNode; children?: ReactNode }) {
  return (
    <div className="topbar">
      <div>{crumb && <div className="crumb">{crumb}</div>}<h1>{title}</h1></div>
      <div className="row right">{children}</div>
    </div>
  );
}
