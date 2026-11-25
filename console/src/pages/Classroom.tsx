import { useMemo, useRef, useState, type ReactNode } from "react";
import {
  AppWindow, Ban, BookOpen, Cast, ChevronDown, ClipboardCheck, Eye, FileUp, Globe, KeyRound, LayoutGrid, List,
  Lock, MessageSquare, MonitorOff, MousePointer2, Pencil, Power, Search, ShieldAlert, ShieldCheck, ShieldOff, Trash2,
  Unlock, UserX, Wand2, WifiOff, X, Armchair, Rocket, Download, RefreshCw,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, displayName, NO_POLICY, useStore, type Policy, type Student } from "../api";
import { Confirm, Empty, Menu, Modal, Switch, errText, time, toast } from "../components/ui";
import { Viewer } from "./Viewer";

type Tab = "all" | "online" | "offline" | "pending";
type View = "grid" | "plan" | "list";
type MenuKind = "lock" | "show" | "web" | "apps" | "power" | "ctx";
type Dlg =
  | { k: "message" } | { k: "lock" } | { k: "url" } | { k: "run" } | { k: "assign" } | { k: "close" } | { k: "rename"; s: Student }
  | { k: "confirm"; title: string; text: ReactNode; danger?: boolean; ok: string; run: () => Promise<unknown> | void };

function natural(a: string, b: string) {
  return a.localeCompare(b, "vi", { numeric: true, sensitivity: "base" });
}

const SIZES = { s: "170px", m: "230px", l: "320px" } as const;

export function Classroom({ openSettings, goTasks }: { openSettings: (section: string) => void; goTasks: () => void }) {
  const st = useStore();
  const { students, thumbs, settings, policy, showing, pending } = st;
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [tab, setTab] = useState<Tab>("all");
  const [q, setQ] = useState("");
  const [sort, setSort] = useState<"name" | "user" | "status">("name");
  const [view, setView] = useState<View>("grid");
  const [size, setSize] = useState<keyof typeof SIZES>("m");
  const [viewer, setViewer] = useState<Student | null>(null);
  const [menu, setMenu] = useState<{ k: MenuKind; a: HTMLElement; s?: Student } | null>(null);
  const [dlg, setDlg] = useState<Dlg | null>(null);
  const [showPw, setShowPw] = useState(false);
  const [hideUpd, setHideUpd] = useState(false);

  const all = useMemo(() => [...students.values()], [students]);
  const online = all.filter((s) => s.state === "online");
  const outdated = online.filter((s) => s.info.version && s.info.version !== st.version);
  const askUpdate = (list: Student[]) => setDlg({
    k: "confirm", title: `Cập nhật ${list.length} máy học sinh lên ${st.version}?`, ok: "Cập nhật",
    text: <>Máy giáo viên gửi phần mềm mới xuống, máy học sinh tự cài và khởi động lại LabControl (khoảng 10–30 giây,
      học sinh không cần làm gì). Bài đang làm không bị ảnh hưởng.</>,
    run: async () => { const n = await api.pushUpdate(list.map((s) => s.id), null); toast(`Đang gửi bản cập nhật tới ${n} máy`); },
  });
  const list = useMemo(() => {
    const k = q.trim().toLowerCase();
    let v = all.filter((s) => tab === "all" || (tab === "online" ? s.state === "online" : s.state === "offline"));
    if (k) v = v.filter((s) => [displayName(s), s.info.name, s.info.user, s.status?.user, s.addr, s.status?.app]
      .some((x) => (x || "").toLowerCase().includes(k)));
    return v.sort((a, b) => sort === "status" ? (a.state === b.state ? natural(displayName(a), displayName(b)) : a.state === "online" ? -1 : 1)
      : sort === "user" ? natural(a.status?.user || a.info.user || "~", b.status?.user || b.info.user || "~")
      : natural(displayName(a), displayName(b)));
  }, [all, tab, q, sort]);

  // lệnh áp dụng cho máy đang chọn; chưa chọn máy nào = cả lớp
  const chosen = online.filter((s) => sel.has(s.id));
  const targets = (chosen.length ? chosen : online).map((s) => s.id);
  const targetNote = chosen.length ? `Áp dụng cho ${chosen.length} máy đã chọn` : `Áp dụng cho cả lớp (${online.length} máy)`;
  const anyLocked = online.some((s) => s.status?.locked);
  const policyOn = JSON.stringify(policy) !== JSON.stringify(NO_POLICY);

  const run = async (label: string, f: () => Promise<unknown>) => {
    try {
      const r = await f();
      toast(typeof r === "number" ? `${label}: ${r} máy` : label);
    } catch (e) { toast(errText(e), "error"); }
  };
  const needOnline = () => {
    if (!targets.length) { toast("Chưa có máy học sinh nào trực tuyến", "error"); return false; }
    return true;
  };
  const setPol = (p: Policy, label: string) => run(label, () => api.setPolicy(p));
  const toggle = (id: string) => setSel((s) => { const n = new Set(s); if (n.has(id)) n.delete(id); else n.add(id); return n; });
  const allSelected = list.length > 0 && list.every((s) => sel.has(s.id));
  const openMenu = (k: MenuKind) => (e: React.MouseEvent<HTMLElement>) => setMenu({ k, a: e.currentTarget });

  const tool = (icon: ReactNode, label: string, onClick: (e: React.MouseEvent<HTMLElement>) => void, opt: { on?: boolean; tone?: "warn" | "err"; caret?: boolean; title?: string } = {}) => (
    <button className={`tool ${opt.on ? "on" : ""} ${opt.tone ? "t-" + opt.tone : ""}`} onClick={onClick} title={opt.title}>
      <span className="ic">{icon}</span>{label}{opt.caret && <ChevronDown size={12} className="caret" />}
    </button>
  );

  const sendFiles = async () => {
    if (!needOnline()) return;
    const p = await open({ multiple: true, title: "Chọn file gửi cho học sinh" });
    const paths = Array.isArray(p) ? p : p ? [p] : [];
    if (!paths.length) return;
    const n = await api.sendFiles(targets, paths.map(String));
    toast(`Đang gửi ${paths.length} file tới ${n / paths.length} máy (thư mục TuGiaoVien trên màn hình nền)`);
  };

  const ctxMenu = (s: Student) => (e: React.MouseEvent) => {
    e.preventDefault();
    const a = { getBoundingClientRect: () => ({ left: e.clientX, bottom: e.clientY, top: e.clientY, right: e.clientX }) } as unknown as HTMLElement;
    setMenu({ k: "ctx", a, s });
  };

  return (
    <>
      <div className="head">
        <div className="title"><h1>{settings?.room ? `Phòng ${settings.room}` : "Lớp học"}<small>{settings?.teacher_name}</small></h1></div>
        <span className="chip ok"><span className="dot on" />{online.length}/{all.length} trực tuyến</span>
        <button className="chip" title="Học sinh nhập mật khẩu này khi cài LabControl" onClick={() => setShowPw(!showPw)}>
          <KeyRound size={13} />Mật khẩu phòng: <b className="mono">{showPw ? settings?.room_password : "••••••"}</b></button>
        {!settings?.accept_new && <span className="chip warn">Đang tắt nhận máy mới</span>}
        {showing && <button className="chip ok" onClick={() => api.stopShow()}><Cast size={13} />Đang trình chiếu · Dừng</button>}
        {policy.exam && <span className="chip warn"><ShieldAlert size={13} />Chế độ thi</span>}
        {policy.block_web.length > 0 && <span className="chip warn"><Globe size={13} />Chặn {policy.block_web.length} trang web</span>}
        {(policy.block_tools || policy.block_apps.length > 0) && !policy.exam && <span className="chip warn"><AppWindow size={13} />Chặn ứng dụng</span>}
        {outdated.length > 0 && <button className="chip warn" title={`Máy giáo viên đang dùng bản ${st.version}`} onClick={() => askUpdate(outdated)}>
          <RefreshCw size={13} />{outdated.length} máy dùng bản cũ · Cập nhật</button>}
        <div className="right" />
      </div>

      <div className="toolbar">
        {tool(anyLocked ? <Lock size={18} /> : <Lock size={18} />, "Khóa máy", openMenu("lock"), { on: anyLocked, tone: "err", caret: true })}
        {tool(<Cast size={18} />, "Trình chiếu", openMenu("show"), { on: showing, caret: true })}
        {tool(<MessageSquare size={18} />, "Tin nhắn", () => needOnline() && setDlg({ k: "message" }))}
        {tool(<FileUp size={18} />, "Gửi file", sendFiles)}
        {tool(<Globe size={18} />, "Mở web", () => needOnline() && setDlg({ k: "url" }))}
        {tool(<Rocket size={18} />, "Mở ứng dụng", () => needOnline() && setDlg({ k: "run" }))}
        <div className="sep" />
        {tool(<BookOpen size={18} />, "Bài tập", () => needOnline() && setDlg({ k: "assign" }))}
        {tool(<ClipboardCheck size={18} />, "Trắc nghiệm", goTasks)}
        <div className="sep" />
        {tool(<Ban size={18} />, "Chặn web", openMenu("web"), { on: policy.block_web.length > 0, tone: "warn", caret: true })}
        {tool(<AppWindow size={18} />, "Chặn ứng dụng", openMenu("apps"), { on: policy.block_tools || policy.block_apps.length > 0, tone: "warn", caret: true })}
        {tool(<X size={18} />, "Đóng ứng dụng", () => needOnline() && setDlg({ k: "close" }))}
        {tool(<ShieldAlert size={18} />, "Chế độ thi", () => setDlg({
          k: "confirm", title: policy.exam ? "Kết thúc chế độ thi?" : "Bật chế độ thi cho cả lớp?",
          text: policy.exam ? "Máy học sinh được dùng lại trình duyệt và các ứng dụng." :
            <>Máy học sinh sẽ hiện dải <b>“Đang trong giờ kiểm tra”</b>; trình duyệt, ứng dụng nhắn tin, điều khiển từ xa
              và các công cụ hệ thống (cmd, PowerShell, Task Manager...) bị đóng ngay khi mở. Các trang web đang chặn vẫn giữ nguyên.</>,
          ok: policy.exam ? "Kết thúc" : "Bật chế độ thi",
          run: () => setPol({ ...policy, exam: !policy.exam }, policy.exam ? "Đã kết thúc chế độ thi" : "Đã bật chế độ thi"),
        }), { on: policy.exam, tone: "warn" })}
        <div className="sep" />
        {tool(<Power size={18} />, "Nguồn", openMenu("power"), { caret: true })}
        {policyOn && tool(<ShieldOff size={18} />, "Bỏ giới hạn", () => setPol(NO_POLICY, "Đã bỏ mọi giới hạn"))}
        <span className="target-note">{targetNote}</span>
      </div>

      {st.update?.newer && !hideUpd && (
        <div className="banner info"><Download size={16} />Đã có LabControl <b>{st.update.version}</b> (bạn đang dùng {st.version}).
          <div className="row right">
            <button className="btn sm primary" onClick={() => api.openRelease(st.update!.url).catch((e) => toast(errText(e), "error"))}>Xem & tải về</button>
            <button className="btn sm ghost" onClick={() => setHideUpd(true)}>Ẩn</button>
          </div></div>
      )}
      {st.listenError && <div className="banner err"><WifiOff size={16} />{st.listenError}</div>}
      {pending.length > 0 && tab !== "pending" && (
        <div className="banner warn"><UserX size={16} />{pending.length} máy học sinh chưa vào được lớp (chưa ghép hoặc sai mật khẩu phòng).
          <button className="btn sm right" onClick={() => setTab("pending")}>Xem</button></div>
      )}

      <div className="filters">
        <input type="checkbox" className="check" title="Chọn tất cả" checked={allSelected}
               onChange={() => setSel(allSelected ? new Set() : new Set(list.map((s) => s.id)))} />
        <div className="tabs">
          <button className={tab === "all" ? "active" : ""} onClick={() => setTab("all")}>Tất cả<b>{all.length}</b></button>
          <button className={tab === "online" ? "active" : ""} onClick={() => setTab("online")}>Trực tuyến<b>{online.length}</b></button>
          <button className={tab === "offline" ? "active" : ""} onClick={() => setTab("offline")}>Ngoại tuyến<b>{all.length - online.length}</b></button>
          <button className={tab === "pending" ? "active" : ""} onClick={() => setTab("pending")}>Chưa ghép<b>{pending.length}</b></button>
        </div>
        <div className="search"><Search size={15} /><input placeholder="Tìm máy, học sinh, ứng dụng..." value={q} onChange={(e) => setQ(e.target.value)} />
          {q && <X size={14} style={{ cursor: "pointer" }} onClick={() => setQ("")} />}</div>
        {sel.size > 0 && <button className="btn ghost sm" onClick={() => setSel(new Set())}>Bỏ chọn ({sel.size})</button>}
        <div className="row right">
          <select className="sel" value={sort} onChange={(e) => setSort(e.target.value as typeof sort)} title="Sắp xếp">
            <option value="name">Theo tên máy</option><option value="user">Theo học sinh</option><option value="status">Trực tuyến trước</option>
          </select>
          {view !== "list" && <div className="seg" title="Cỡ ảnh">
            {(["s", "m", "l"] as const).map((k) => <button key={k} className={size === k ? "active" : ""} onClick={() => setSize(k)}>
              {k === "s" ? "S" : k === "m" ? "M" : "L"}</button>)}
          </div>}
          <div className="seg">
            <button className={view === "grid" ? "active" : ""} onClick={() => setView("grid")} title="Lưới màn hình"><LayoutGrid size={16} /></button>
            <button className={view === "plan" ? "active" : ""} onClick={() => setView("plan")} title="Sơ đồ phòng"><Armchair size={16} /></button>
            <button className={view === "list" ? "active" : ""} onClick={() => setView("list")} title="Danh sách"><List size={16} /></button>
          </div>
        </div>
      </div>

      <div className="content" style={{ ["--tile" as string]: SIZES[size] }}>
        {tab === "pending" ? <PendingList /> : all.length === 0 ? <Setup openSettings={openSettings} /> :
          view === "grid" ? (
            list.length === 0 ? <Empty icon={<Search size={22} />} title="Không có máy nào khớp" /> :
              <div className="grid">{list.map((s) => <Tile key={s.id} s={s} thumb={thumbs.get(s.id)} sel={sel.has(s.id)}
                onClick={() => toggle(s.id)} onOpen={() => s.state === "online" && setViewer(s)} onCtx={ctxMenu(s)} />)}</div>
          ) : view === "plan" ? (
            <SeatPlan list={list} sel={sel} thumbs={thumbs} toggle={toggle} open={(s) => s.state === "online" && setViewer(s)} ctx={ctxMenu} />
          ) : (
            <table className="table">
              <thead><tr><th style={{ width: 34 }} /><th>Máy</th><th>Học sinh</th><th>Trạng thái</th><th>Đang dùng</th><th>Địa chỉ</th><th>Phiên bản</th></tr></thead>
              <tbody>{list.map((s) => (
                <tr key={s.id} className={`click ${sel.has(s.id) ? "sel" : ""}`} onClick={() => toggle(s.id)} onDoubleClick={() => s.state === "online" && setViewer(s)} onContextMenu={ctxMenu(s)}>
                  <td><input type="checkbox" className="check" checked={sel.has(s.id)} onChange={() => toggle(s.id)} onClick={(e) => e.stopPropagation()} /></td>
                  <td><b>{displayName(s)}</b>{s.label && <div className="small faint">{s.info.name}</div>}</td>
                  <td>{s.status?.user || s.info.user || "—"}</td>
                  <td><span className="row"><span className={`dot ${s.state === "online" ? "on" : ""}`} />{s.state === "online" ? "Trực tuyến" : "Ngoại tuyến"}
                    <Flags s={s} /></span></td>
                  <td className="small">{s.status?.app || "—"}{s.status?.title ? <span className="faint"> · {s.status.title.slice(0, 50)}</span> : null}</td>
                  <td className="mono">{s.addr}</td>
                  <td className="small faint">{s.info.version || "—"}{s.info.os ? ` · ${s.info.os}` : ""}</td>
                </tr>))}</tbody>
            </table>
          )}
      </div>

      {/* ------------------------------------------------ menu */}
      {menu?.k === "lock" && <Menu anchor={menu.a} onClose={() => setMenu(null)}>
        <button onClick={() => { setMenu(null); if (needOnline()) setDlg({ k: "lock" }); }}><Lock size={16} />Khóa màn hình...</button>
        <button onClick={() => { setMenu(null); if (needOnline()) run("Đã mở khóa", () => api.send(targets, { t: "unlock" })); }}><Unlock size={16} />Mở khóa</button>
        <hr /><div className="mh">{targetNote}</div>
      </Menu>}
      {menu?.k === "show" && <Menu anchor={menu.a} onClose={() => setMenu(null)}>
        <div className="mh">Chiếu màn hình của bạn cho học sinh</div>
        <button onClick={() => { setMenu(null); if (needOnline()) run("Đang trình chiếu toàn màn hình", () => api.startShow(targets, true)); }}><Cast size={16} />Toàn màn hình (khóa máy học sinh)</button>
        <button onClick={() => { setMenu(null); if (needOnline()) run("Đang trình chiếu trong cửa sổ", () => api.startShow(targets, false)); }}><AppWindow size={16} />Trong cửa sổ</button>
        {showing && <><hr /><button className="danger" onClick={() => { setMenu(null); api.stopShow(); }}><MonitorOff size={16} />Dừng trình chiếu</button></>}
      </Menu>}
      {menu?.k === "web" && <WebMenu anchor={menu.a} onClose={() => setMenu(null)} policy={policy} list={settings?.web_list ?? []}
                                     apply={(p) => setPol(p, p.block_web.length ? `Đang chặn ${p.block_web.length} trang web` : "Đã bỏ chặn web")}
                                     edit={() => { setMenu(null); openSettings("lists"); }} />}
      {menu?.k === "apps" && <AppsMenu anchor={menu.a} onClose={() => setMenu(null)} policy={policy} list={settings?.app_list ?? []}
                                       apply={(p) => setPol(p, "Đã cập nhật chặn ứng dụng")} edit={() => { setMenu(null); openSettings("lists"); }} />}
      {menu?.k === "power" && <Menu anchor={menu.a} onClose={() => setMenu(null)}>
        {([["shutdown", "Tắt máy"], ["restart", "Khởi động lại"], ["logoff", "Đăng xuất"]] as const).map(([a, l]) => (
          <button key={a} onClick={() => { setMenu(null); if (needOnline()) setDlg({ k: "confirm", title: `${l}?`, danger: true, ok: l,
            text: `${l} ${targets.length} máy học sinh. Bài đang làm chưa lưu có thể bị mất.`, run: () => api.send(targets, { t: "power", action: a }) }); }}>
            <Power size={16} />{l}</button>))}
        <hr /><div className="mh">{targetNote}</div>
      </Menu>}
      {menu?.k === "ctx" && menu.s && (() => {
        const s = menu.s; const id = [s.id]; const on = s.state === "online";
        return <Menu anchor={menu.a} onClose={() => setMenu(null)}>
          <div className="mh">{displayName(s)}</div>
          {on && <>
            <button onClick={() => { setMenu(null); setViewer(s); }}><Eye size={16} />Xem / điều khiển</button>
            <button onClick={() => { setMenu(null); api.send(id, { t: "lock", message: "Hãy chú ý lên giáo viên" }); }}><Lock size={16} />Khóa</button>
            <button onClick={() => { setMenu(null); api.send(id, { t: "unlock" }); }}><Unlock size={16} />Mở khóa</button>
            <button onClick={() => { setMenu(null); setSel(new Set(id)); setDlg({ k: "message" }); }}><MessageSquare size={16} />Gửi tin nhắn</button>
          </>}
          {on && s.info.version !== st.version && <button onClick={() => { setMenu(null); askUpdate([s]); }}><RefreshCw size={16} />Cập nhật phần mềm ({s.info.version || "?"} → {st.version})</button>}
          <button onClick={() => { setMenu(null); setDlg({ k: "rename", s }); }}><Pencil size={16} />Đặt tên hiển thị</button>
          {!on && <button className="danger" onClick={() => { setMenu(null); setDlg({ k: "confirm", title: "Gỡ máy khỏi danh sách?", danger: true, ok: "Gỡ",
            text: "Máy này sẽ hiện lại khi tự nối về. Muốn máy không nối về nữa, hãy gỡ LabControl trên máy đó.", run: () => api.remove(s.id) }); }}>
            <Trash2 size={16} />Gỡ khỏi danh sách</button>}
        </Menu>;
      })()}

      {/* ------------------------------------------------ hộp thoại */}
      {dlg?.k === "message" && <TextDialog title="Gửi tin nhắn" label="Nội dung" note={targetNote} multiline ok="Gửi"
        onClose={() => setDlg(null)} onOk={(t) => run("Đã gửi tin nhắn", () => api.send(targets, { t: "message", text: t }))} />}
      {dlg?.k === "lock" && <TextDialog title="Khóa màn hình học sinh" label="Dòng chữ trên màn hình khóa" initial="Hãy chú ý lên giáo viên"
        note={targetNote} ok="Khóa" onClose={() => setDlg(null)} onOk={(t) => run("Đã khóa", () => api.send(targets, { t: "lock", message: t }))} />}
      {dlg?.k === "url" && <TextDialog title="Mở trang web trên máy học sinh" label="Địa chỉ" initial="https://" note={targetNote} ok="Mở"
        onClose={() => setDlg(null)} onOk={(t) => {
          const url = /^https?:\/\//.test(t) ? t : `https://${t}`;
          return run("Đã mở trang web", () => api.send(targets, { t: "open_url", url }));
        }} />}
      {dlg?.k === "run" && <RunAppDialog note={targetNote} custom={settings?.run_list ?? []}
        oses={online.filter((s) => targets.includes(s.id)).map((s) => s.info.os)}
        onClose={() => setDlg(null)} onEdit={() => { setDlg(null); openSettings("lists"); }}
        onOk={(c) => run(`Đã mở ${c}`, () => api.send(targets, { t: "run_app", cmd: c }))} />}
      {dlg?.k === "close" && <CloseAppDialog note={targetNote} apps={[...new Set(online.filter((s) => targets.includes(s.id)).map((s) => s.status?.app).filter(Boolean) as string[])]}
        onClose={() => setDlg(null)} onOk={(n) => run(`Đã yêu cầu đóng ${n}`, () => api.send(targets, { t: "close_app", name: n }))} />}
      {dlg?.k === "assign" && <AssignDialog note={targetNote} onClose={() => setDlg(null)}
        onOk={async (title, text) => { await api.assign(targets, title, text); toast(`Đã giao bài “${title}” cho ${targets.length} máy`); }} />}
      {dlg?.k === "rename" && <TextDialog title={`Tên hiển thị – ${dlg.s.info.name}`} label="Tên (vd: tên học sinh ngồi máy này)" initial={dlg.s.label}
        ok="Lưu" allowEmpty onClose={() => setDlg(null)} onOk={(t) => api.setLabel(dlg.s.id, t)} />}
      {dlg?.k === "confirm" && <Confirm title={dlg.title} message={dlg.text} danger={dlg.danger} confirmText={dlg.ok}
        onClose={() => setDlg(null)} onConfirm={async () => { await dlg.run(); }} />}

      {viewer && <Viewer s={viewer} onClose={() => setViewer(null)} />}
    </>
  );
}

// ------------------------------------------------------------------ ô màn hình
function Flags({ s }: { s: Student }) {
  const { policy } = useStore();
  const t = s.status;
  if (!t || s.state !== "online") return null;
  return <>
    {t.locked && <span className="flag err" title="Đang khóa"><Lock size={12} /></span>}
    {t.viewed && <span className="flag err" title="Đang được xem"><Eye size={12} /></span>}
    {t.showing && <span className="flag pri" title="Đang xem trình chiếu"><Cast size={12} /></span>}
    {t.exam && <span className="flag warn" title="Chế độ thi"><ShieldAlert size={12} /></span>}
    {t.web_blocked && !t.exam && <span className="flag warn" title="Đang chặn web"><Ban size={12} /></span>}
    {!t.helper && policy.block_web.length > 0 && <span className="flag" title="Máy chưa cài dịch vụ Helper: không chặn web được"><ShieldOff size={12} /></span>}
  </>;
}

function Tile({ s, thumb, sel, onClick, onOpen, onCtx, draggable }: {
  s: Student; thumb?: string; sel: boolean; onClick: () => void; onOpen: () => void; onCtx: (e: React.MouseEvent) => void; draggable?: boolean;
}) {
  const on = s.state === "online";
  const user = s.status?.user || s.info.user;
  return (
    <div className={`tile ${sel ? "sel" : ""} ${on ? "" : "off"}`} onClick={onClick} onDoubleClick={onOpen} onContextMenu={onCtx}
         draggable={draggable} onDragStart={(e) => e.dataTransfer.setData("text/plain", s.id)}>
      <div className="shot">
        {on && thumb ? <img src={thumb} alt="" draggable={false} /> : <div className="ph">{on ? <div className="spinner" /> : <MonitorOff size={26} />}</div>}
        <div className="flags"><Flags s={s} /></div>
        <input type="checkbox" className="check ck" checked={sel} onChange={onClick} onClick={(e) => e.stopPropagation()} />
      </div>
      <div className="meta">
        <span className={`dot ${on ? "on" : ""}`} />
        <div style={{ minWidth: 0 }}>
          <div className="nm">{displayName(s)}</div>
          <div className="us">{user ? user : on ? "Chưa đăng nhập" : "Ngoại tuyến"}</div>
        </div>
        {on && <button className="btn ghost sm right" title="Xem / điều khiển" onClick={(e) => { e.stopPropagation(); onOpen(); }}><MousePointer2 size={14} /></button>}
      </div>
      {on && s.status?.app && <div className="cur-app" title={s.status.title}>{s.status.app}</div>}
    </div>
  );
}

// ------------------------------------------------------------------ sơ đồ phòng
function SeatPlan({ list, sel, thumbs, toggle, open, ctx }: {
  list: Student[]; sel: Set<string>; thumbs: Map<string, string>; toggle: (id: string) => void; open: (s: Student) => void;
  ctx: (s: Student) => (e: React.MouseEvent) => void;
}) {
  const { settings } = useStore();
  const cols = Math.max(2, settings?.seat_cols || 6);
  const [over, setOver] = useState<string | null>(null);
  const placed = new Map<string, Student>();
  for (const s of list) if (s.seat) placed.set(`${s.seat[0]},${s.seat[1]}`, s);
  const maxRow = Math.max(-1, ...list.filter((s) => s.seat).map((s) => s.seat![1]));
  const rows = Math.max(3, maxRow + 2);
  const unplaced = list.filter((s) => !s.seat);
  const setCols = async (n: number) => {
    if (!settings) return;
    await api.setSettings({ ...settings, seat_cols: Math.min(14, Math.max(2, n)) }).catch((e) => toast(errText(e), "error"));
    const { reload } = await import("../api");
    reload();
  };
  const drop = (seat: [number, number] | null) => (e: React.DragEvent) => {
    e.preventDefault(); setOver(null);
    const id = e.dataTransfer.getData("text/plain");
    if (id) api.setSeat(id, seat);
  };
  return (
    <div className="plan">
      <div className="row">
        <span className="small muted">Kéo thả máy vào chỗ ngồi giống phòng thật. Thả lên máy khác để đổi chỗ.</span>
        <div className="row right">
          <span className="small">Số cột</span>
          <button className="btn sm" onClick={() => setCols(cols - 1)}>−</button><b>{cols}</b>
          <button className="btn sm" onClick={() => setCols(cols + 1)}>+</button>
          <button className="btn sm" onClick={() => api.autoSeats(cols)}><Wand2 size={14} />Xếp tự động theo tên</button>
        </div>
      </div>
      <div className="desk">Bàn giáo viên</div>
      <div className="seats" style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {Array.from({ length: rows * cols }, (_, i) => {
          const c = i % cols, r = Math.floor(i / cols), k = `${c},${r}`;
          const s = placed.get(k);
          return <div key={k} className={`seat ${over === k ? "over" : ""}`} onDragOver={(e) => { e.preventDefault(); setOver(k); }}
                      onDragLeave={() => setOver(null)} onDrop={drop([c, r])}>
            {s && <Tile s={s} thumb={thumbs.get(s.id)} sel={sel.has(s.id)} onClick={() => toggle(s.id)} onOpen={() => open(s)} onCtx={ctx(s)} draggable />}
          </div>;
        })}
      </div>
      <div className={`tray ${over === "tray" ? "over" : ""}`} onDragOver={(e) => { e.preventDefault(); setOver("tray"); }} onDragLeave={() => setOver(null)} onDrop={drop(null)}>
        <div className="small muted" style={{ marginBottom: 8 }}>Chưa xếp chỗ ({unplaced.length}) – kéo lên sơ đồ</div>
        {unplaced.length ? <div className="grid">{unplaced.map((s) => <Tile key={s.id} s={s} thumb={thumbs.get(s.id)} sel={sel.has(s.id)}
          onClick={() => toggle(s.id)} onOpen={() => open(s)} onCtx={ctx(s)} draggable />)}</div> : <div className="small faint">Mọi máy đã có chỗ.</div>}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ máy chưa vào được lớp
function PendingList() {
  const { pending, settings } = useStore();
  if (!pending.length) return <Empty icon={<ShieldCheck size={22} />} title="Không có máy nào bị từ chối">
    <div className="small muted">Máy nhập sai mật khẩu phòng hoặc chưa được ghép sẽ hiện ở đây.</div></Empty>;
  return <div className="col" style={{ gap: 12 }}>
    <div className="banner info" style={{ margin: 0 }}>Máy dưới đây đã tìm thấy máy giáo viên nhưng chưa vào được lớp.
      Hãy kiểm tra mật khẩu phòng trên máy học sinh khớp với <b className="mono">&nbsp;{settings?.room_password}&nbsp;</b>
      {!settings?.accept_new && <> và bật “Nhận máy mới” trong Cài đặt</>}.</div>
    <table className="table">
      <thead><tr><th>Máy</th><th>Người dùng</th><th>Địa chỉ</th><th>Lý do</th><th>Lúc</th><th /></tr></thead>
      <tbody>{pending.map((p) => <tr key={p.id}>
        <td><b>{p.name}</b></td><td>{p.user || "—"}</td><td className="mono">{p.ip}</td>
        <td className="small" style={{ color: "var(--warn)" }}>{p.reason}</td><td className="small faint">{time(p.at)}</td>
        <td><button className="btn ghost sm" onClick={() => api.dismissPending(p.id)}>Ẩn</button></td></tr>)}</tbody>
    </table>
  </div>;
}

// ------------------------------------------------------------------ hướng dẫn khi chưa có máy
function Setup({ openSettings }: { openSettings: (s: string) => void }) {
  const { settings, addresses } = useStore();
  const pw = settings?.room_password ?? "";
  const room = settings?.room ? ` --room "${settings.room}"` : "";
  const cmd = `labcontrol-agent.exe install --password "${pw}"${room}`;
  return <div className="card setup">
    <div className="card-h"><h2>Đưa máy học sinh vào lớp</h2></div>
    <div className="card-b">
      <div className="steps">
        <div className="step"><div className="col">
          <b>Máy giáo viên đã sẵn sàng</b>
          <span className="small muted">Máy học sinh trong cùng mạng LAN sẽ tự tìm thấy máy này{addresses.length ? <> (địa chỉ <span className="mono">{addresses.join(", ")}</span>)</> : null}.
            Nếu Windows hỏi cho phép LabControl qua tường lửa, hãy chọn <b>Cho phép</b>.</span>
        </div></div>
        <div className="step"><div className="col">
          <b>Mật khẩu phòng: <span className="pw">{pw}</span></b>
          <span className="small muted">Học sinh/kỹ thuật viên nhập mật khẩu này một lần khi cài. Đổi trong <a href="#" onClick={(e) => { e.preventDefault(); openSettings("room"); }}>Cài đặt</a>.</span>
        </div></div>
        <div className="step"><div className="col" style={{ minWidth: 0 }}>
          <b>Cài LabControl trên từng máy học sinh (quyền quản trị)</b>
          <span className="small muted">Chạy <span className="mono">cai-may-hoc-sinh.cmd</span> và nhập mật khẩu phòng, hoặc dùng lệnh:</span>
          <div className="code-box">{cmd}</div>
          <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => navigator.clipboard.writeText(cmd).then(() => toast("Đã chép lệnh"))}>Chép lệnh</button>
        </div></div>
        <div className="step"><div className="col">
          <b>Học sinh đăng nhập – máy tự hiện ở đây</b>
          <span className="small muted">Lần đầu máy ghép bằng mật khẩu phòng, các lần sau tự vào lớp. Máy học sinh không cần mở tường lửa.</span>
        </div></div>
      </div>
    </div>
  </div>;
}

// ------------------------------------------------------------------ menu chặn
function WebMenu({ anchor, onClose, policy, list, apply, edit }: {
  anchor: HTMLElement; onClose: () => void; policy: Policy; list: string[]; apply: (p: Policy) => void; edit: () => void;
}) {
  const [on, setOn] = useState<Set<string>>(new Set(policy.block_web));
  const [extra, setExtra] = useState("");
  const all = [...new Set([...list, ...policy.block_web])];
  const add = () => {
    const d = extra.trim().replace(/^https?:\/\//, "").split("/")[0].toLowerCase();
    if (d) { setOn((s) => new Set(s).add(d)); setExtra(""); }
  };
  return <Menu anchor={anchor} onClose={onClose} width={340}>
    <div className="mh">Chặn trang web – cả lớp</div>
    <div style={{ padding: "4px 10px 8px" }} className="col">
      <div className="pills">{[...new Set([...all, ...on])].map((d) => <button key={d} className={`pill ${on.has(d) ? "on" : ""}`}
        onClick={() => setOn((s) => { const n = new Set(s); if (n.has(d)) n.delete(d); else n.add(d); return n; })}>{d}</button>)}</div>
      <div className="row"><input className="input" placeholder="Thêm tên miền, vd: game.vn" value={extra} onChange={(e) => setExtra(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && add()} /><button className="btn sm" onClick={add}>Thêm</button></div>
      <div className="small faint">Chặn cả www. và m. của tên miền. Cần dịch vụ Helper trên máy học sinh (cài bằng lệnh install).</div>
      <div className="row">
        <button className="btn sm ghost" onClick={edit}>Sửa danh sách...</button>
        <button className="btn sm right" onClick={() => { apply({ ...policy, block_web: [] }); onClose(); }}>Bỏ chặn</button>
        <button className="btn sm primary" onClick={() => { apply({ ...policy, block_web: [...on] }); onClose(); }}>Chặn {on.size} trang</button>
      </div>
    </div>
  </Menu>;
}

function AppsMenu({ anchor, onClose, policy, list, apply, edit }: {
  anchor: HTMLElement; onClose: () => void; policy: Policy; list: string[]; apply: (p: Policy) => void; edit: () => void;
}) {
  const [tools, setTools] = useState(policy.block_tools);
  const [on, setOn] = useState<Set<string>>(new Set(policy.block_apps));
  const all = [...new Set([...list, ...policy.block_apps])];
  return <Menu anchor={anchor} onClose={onClose} width={340}>
    <div className="mh">Chặn ứng dụng – cả lớp</div>
    <div style={{ padding: "4px 10px 8px" }} className="col">
      <Switch checked={tools} onChange={setTools} label="Chặn công cụ hệ thống" hint="cmd, PowerShell, Task Manager, Registry, Terminal..." />
      <div className="small muted" style={{ marginTop: 6 }}>Đóng ngay khi mở:</div>
      <div className="pills">{all.map((d) => <button key={d} className={`pill ${on.has(d) ? "on" : ""}`}
        onClick={() => setOn((s) => { const n = new Set(s); if (n.has(d)) n.delete(d); else n.add(d); return n; })}>{d}</button>)}</div>
      <div className="row">
        <button className="btn sm ghost" onClick={edit}>Sửa danh sách...</button>
        <button className="btn sm primary right" onClick={() => { apply({ ...policy, block_tools: tools, block_apps: [...on] }); onClose(); }}>Áp dụng</button>
      </div>
    </div>
  </Menu>;
}

// ------------------------------------------------------------------ hộp thoại nhập
function TextDialog({ title, label, initial = "", note, ok, multiline, allowEmpty, onClose, onOk }: {
  title: string; label: string; initial?: string; note?: string; ok: string; multiline?: boolean; allowEmpty?: boolean;
  onClose: () => void; onOk: (t: string) => unknown;
}) {
  const [t, setT] = useState(initial);
  const ref = useRef<HTMLTextAreaElement & HTMLInputElement>(null);
  const go = async () => {
    if (!t.trim() && !allowEmpty) return;
    try { await onOk(t.trim()); onClose(); } catch (e) { toast(errText(e), "error"); }
  };
  return <Modal title={title} onClose={onClose} footer={<><button className="btn" onClick={onClose}>Hủy</button>
    <button className="btn primary" disabled={!t.trim() && !allowEmpty} onClick={go}>{ok}</button></>}>
    <div className="field"><label>{label}</label>
      {multiline ? <textarea ref={ref} autoFocus className="input" rows={4} value={t} onChange={(e) => setT(e.target.value)}
                             onKeyDown={(e) => e.key === "Enter" && e.ctrlKey && go()} />
        : <input ref={ref} autoFocus className="input" value={t} onChange={(e) => setT(e.target.value)} onKeyDown={(e) => e.key === "Enter" && go()} />}
      {note && <span className="hint">{note}{multiline ? " · Ctrl+Enter để gửi" : ""}</span>}</div>
  </Modal>;
}

function CloseAppDialog({ note, apps, onClose, onOk }: { note: string; apps: string[]; onClose: () => void; onOk: (n: string) => unknown }) {
  const [n, setN] = useState("");
  return <Modal title="Đóng ứng dụng trên máy học sinh" onClose={onClose} footer={<><button className="btn" onClick={onClose}>Hủy</button>
    <button className="btn primary" disabled={!n.trim()} onClick={async () => { await onOk(n.trim()); onClose(); }}>Đóng ứng dụng</button></>}>
    <div className="field"><label>Tên tiến trình</label>
      <input className="input" autoFocus placeholder="vd: chrome.exe, zalo.exe, game.exe" value={n} onChange={(e) => setN(e.target.value)} />
      <span className="hint">{note}. Muốn chặn hẳn (mở lại cũng bị đóng), dùng “Chặn ứng dụng”.</span></div>
    {apps.length > 0 && <div className="field"><label>Đang mở trên các máy</label>
      <div className="pills">{apps.map((a) => <button key={a} className={`pill ${n === a ? "on" : ""}`} onClick={() => setN(a)}>{a}</button>)}</div></div>}
  </Modal>;
}

function AssignDialog({ note, onClose, onOk }: { note: string; onClose: () => void; onOk: (title: string, text: string) => Promise<void> }) {
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [err, setErr] = useState("");
  return <Modal title="Giao bài tập" onClose={onClose} footer={<><button className="btn" onClick={onClose}>Hủy</button>
    <button className="btn primary" disabled={!title.trim()} onClick={async () => {
      try { await onOk(title.trim(), text); onClose(); } catch (e) { setErr(errText(e)); }
    }}>Giao bài</button></>}>
    <div className="field"><label>Tên bài</label><input className="input" autoFocus placeholder="vd: Bài 3 – Vòng lặp" value={title} onChange={(e) => setTitle(e.target.value)} /></div>
    <div className="field"><label>Đề bài / hướng dẫn</label><textarea className="input" rows={5} value={text} onChange={(e) => setText(e.target.value)} />
      <span className="hint">Máy học sinh tạo thư mục <b>BaiLam\{title.trim() || "<tên bài>"}</b> trên màn hình nền, kèm file DE-BAI.txt. {note}.
        Thu bài ở trang “Bài & kiểm tra”.</span></div>
    {err && <div className="err">{err}</div>}
  </Modal>;
}

// ------------------------------------------------------------------ mở ứng dụng
const WIN_APPS: [string, string][] = [
  ["Notepad", "notepad"], ["Paint", "mspaint"], ["Máy tính", "calc"], ["Word", "winword"], ["Excel", "excel"],
  ["PowerPoint", "powerpnt"], ["File Explorer", "explorer"], ["Chrome", "chrome"], ["Edge", "msedge"],
];
const LINUX_APPS: [string, string][] = [
  ["Soạn văn bản", "xed"], ["Writer", "libreoffice --writer"], ["Calc", "libreoffice --calc"],
  ["Impress", "libreoffice --impress"], ["Quản lý file", "nemo"], ["Firefox", "firefox"], ["Chrome", "google-chrome"],
];

function RunAppDialog({ note, custom, oses, onClose, onOk, onEdit }: {
  note: string; custom: string[]; oses: string[]; onClose: () => void; onOk: (c: string) => unknown; onEdit: () => void;
}) {
  const [c, setC] = useState("");
  const win = oses.some((o) => /windows/i.test(o));
  const lin = oses.some((o) => !/windows/i.test(o));
  const quick: [string, string][] = [
    ...custom.map((x) => [x.split(/[\\/]/).pop()!.replace(/\.exe"?$/i, "").replace(/^"/, ""), x] as [string, string]),
    ...(win ? WIN_APPS : []), ...(lin ? LINUX_APPS : []),
  ];
  const go = async () => { if (c.trim()) { await onOk(c.trim()); onClose(); } };
  return <Modal title="Mở ứng dụng trên máy học sinh" onClose={onClose} footer={<>
    <button className="btn ghost" style={{ marginRight: "auto" }} onClick={onEdit}>Sửa danh sách...</button>
    <button className="btn" onClick={onClose}>Hủy</button>
    <button className="btn primary" disabled={!c.trim()} onClick={go}>Mở</button></>}>
    <div className="field"><label>Ứng dụng</label>
      <input className="input mono" autoFocus placeholder={win ? "vd: notepad, winword, C:\\...\\Scratch 3.exe" : "vd: firefox, libreoffice --writer"}
             value={c} onChange={(e) => setC(e.target.value)} onKeyDown={(e) => e.key === "Enter" && go()} />
      <span className="hint">Tên chương trình, đường dẫn đầy đủ, hoặc lệnh có tham số. {note}.</span></div>
    {quick.length > 0 && <div className="field"><label>Chọn nhanh</label>
      <div className="pills">{quick.map(([label, cmd]) => <button key={cmd} className={`pill ${c === cmd ? "on" : ""}`} title={cmd} onClick={() => setC(cmd)}>{label}</button>)}</div></div>}
  </Modal>;
}
