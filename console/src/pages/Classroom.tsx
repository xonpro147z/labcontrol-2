import { tr } from "../i18n";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  AppWindow, Ban, BookOpen, Cast, ChevronDown, ClipboardCheck, Eye, FileUp, Globe, KeyRound, LayoutGrid, List,
  Lock, MessageSquare, MonitorOff, MousePointer2, Pencil, Power, Search, ShieldAlert, ShieldCheck, ShieldOff, Trash2,
  Unlock, UserX, Wand2, WifiOff, X, Armchair, Rocket, Download, RefreshCw, FolderUp, UserCog, VolumeX, Volume2,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, displayName, NO_POLICY, useStore, type Policy, type Student } from "../api";
import { Confirm, Empty, Menu, Modal, Switch, errText, time, toast } from "../components/ui";
import { Viewer } from "./Viewer";

type Tab = "all" | "online" | "offline" | "pending";
type View = "grid" | "plan" | "list";
type MenuKind = "lock" | "show" | "web" | "apps" | "power" | "ctx" | "send";
type Dlg =
  | { k: "message" } | { k: "url" } | { k: "run" } | { k: "assign" } | { k: "close" } | { k: "rename"; s: Student }
  | { k: "confirm"; title: string; text: ReactNode; danger?: boolean; ok: string; run: () => Promise<unknown> | void };

function natural(a: string, b: string) {
  return a.localeCompare(b, "vi", { numeric: true, sensitivity: "base" });
}

const SIZES = { s: "170px", m: "230px", l: "320px" } as const;

export function Classroom({ openSettings, goTasks }: { openSettings: (section: string) => void; goTasks: () => void }) {
  const st = useStore();
  const { students, thumbs, settings, policy, overrides, showing, pending } = st;
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
    k: "confirm", title: tr("Cập nhật {0} máy học sinh lên {1}?", list.length, st.version), ok: tr("Cập nhật"),
    text: <>{tr("Máy giáo viên gửi phần mềm mới xuống, máy học sinh tự cài và khởi động lại LabControl (khoảng 10–30 giây, học sinh không cần làm gì). Bài đang làm không bị ảnh hưởng.")}</>,
    run: async () => { const n = await api.pushUpdate(list.map((s) => s.id), null); toast(tr("Đang gửi bản cập nhật tới {0} máy", n)); },
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
  const targetNote = chosen.length ? tr("Áp dụng cho {0} máy đã chọn", chosen.length) : tr("Áp dụng cho cả lớp ({0} máy)", online.length);
  const anyLocked = online.some((s) => s.status?.locked);
  // Nút Khóa máy mở menu: Khóa máy / Mở khóa máy, áp dụng cho máy đang chọn (hoặc cả lớp), không hỏi lại.
  // màn hình khóa chỉ ghi "Máy đã bị giáo viên khóa" (máy học sinh bản cũ hiện thêm dòng này)
  const lockMsg = () => ({ t: "lock" as const, message: tr("Máy đã bị giáo viên khóa") });
  const lockNow = (ids: string[], name?: string) => run(name ? tr("Đã khóa {0}", name) : tr("Đã khóa"), () => api.send(ids, lockMsg()));
  const unlockNow = (ids: string[], name?: string) => run(name ? tr("Đã mở khóa {0}", name) : tr("Đã mở khóa"), () => api.send(ids, { t: "unlock" }));
  const isFree = (p: Policy) => JSON.stringify({ ...NO_POLICY, ...p }) === JSON.stringify(NO_POLICY);
  const nOverride = Object.keys(overrides).length;
  const policyOn = !isFree(policy) || Object.values(overrides).some((p) => !isFree(p));
  // Giới hạn áp dụng cho máy đang chọn (riêng) hay cả lớp (chung)
  const scoped = chosen.length > 0;
  const effOf = (id: string) => overrides[id] ?? policy;
  const curPol = scoped ? effOf(chosen[0].id) : policy;
  const scopeIds = scoped ? chosen.map((s) => s.id) : null;
  const scopeLabel = scoped ? tr("{0} máy đã chọn", chosen.length) : tr("cả lớp");

  const run = async (label: string, f: () => Promise<unknown>) => {
    try {
      const r = await f();
      toast(typeof r === "number" ? tr("{0}: {1} máy", label, r) : label);
    } catch (e) { toast(errText(e), "error"); }
  };
  const needOnline = () => {
    if (!targets.length) { toast(tr("Chưa có máy học sinh nào trực tuyến"), "error"); return false; }
    return true;
  };
  const setPol = (p: Policy, label: string) => run(label, () => api.setPolicy(p, scopeIds));
  const toggle = (id: string) => { anchor.current = id; setSel((s) => { const n = new Set(s); if (n.has(id)) n.delete(id); else n.add(id); return n; }); };
  // Bấm: chọn một máy (bấm lại để bỏ chọn). Ctrl/⌘ + bấm: thêm/bớt máy. Shift + bấm: chọn một dải máy.
  const anchor = useRef<string | null>(null);
  const pick = (id: string, e: { ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }) => {
    if (e.shiftKey && anchor.current) {
      const ids = list.map((s) => s.id);
      const a = ids.indexOf(anchor.current), b = ids.indexOf(id);
      if (a >= 0 && b >= 0) {
        const range = ids.slice(Math.min(a, b), Math.max(a, b) + 1);
        setSel((s) => new Set([...(e.ctrlKey || e.metaKey ? s : []), ...range]));
        return;
      }
    }
    if (e.ctrlKey || e.metaKey) return toggle(id);
    anchor.current = id;
    setSel((s) => s.size === 1 && s.has(id) ? new Set() : new Set([id]));
  };
  const selectAll = () => setSel(new Set(list.map((s) => s.id)));
  // Ctrl/⌘ + A: chọn tất cả máy; Esc: bỏ chọn
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.closest("input, textarea, select, [contenteditable=true]") || t.closest(".modal, .menu"))) return;
      if (viewer || document.querySelector(".modal")) return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") { e.preventDefault(); selectAll(); }
      else if (e.key === "Escape" && !menu) setSel(new Set());
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
  const allSelected = list.length > 0 && list.every((s) => sel.has(s.id));
  const openMenu = (k: MenuKind) => (e: React.MouseEvent<HTMLElement>) => setMenu({ k, a: e.currentTarget });

  const tool = (icon: ReactNode, label: string, onClick: (e: React.MouseEvent<HTMLElement>) => void, opt: { on?: boolean; tone?: "warn" | "err"; caret?: boolean; title?: string } = {}) => (
    <button className={`tool ${opt.on ? "on" : ""} ${opt.tone ? "t-" + opt.tone : ""}`} onClick={onClick} title={opt.title}>
      <span className="ic">{icon}</span>{label}{opt.caret && <ChevronDown size={12} className="caret" />}
    </button>
  );

  const sendPaths = async (paths: string[]) => {
    if (!paths.length || !needOnline()) return;
    try {
      const b = await api.sendFiles(targets, paths);
      toast(tr("Đang gửi {0} file tới {1} máy", b.files, b.machines));
    } catch (e) { toast(errText(e), "error"); }
  };
  const pickFiles = async (directory: boolean) => {
    setMenu(null);
    if (!needOnline()) return;
    const p = await open({ multiple: true, directory, title: directory ? tr("Chọn thư mục gửi cho học sinh") : tr("Chọn file gửi cho học sinh") });
    const paths = (Array.isArray(p) ? p : p ? [p] : []).map(String);
    await sendPaths(paths);
  };
  // kéo file/thư mục từ máy tính thả vào cửa sổ để gửi
  const [dropOver, setDropOver] = useState(false);
  const sendRef = useRef(sendPaths);
  sendRef.current = sendPaths;
  useEffect(() => {
    let off: (() => void) | undefined;
    let alive = true;
    let wv;
    try { wv = getCurrentWebview(); } catch { return; }
    wv.onDragDropEvent((ev) => {
      const t = ev.payload.type;
      if (t === "enter" || t === "over") setDropOver(true);
      else if (t === "leave") setDropOver(false);
      else if (t === "drop") { setDropOver(false); sendRef.current(ev.payload.paths); }
    }).then((f) => { if (alive) off = f; else f(); }).catch(() => {});
    return () => { alive = false; off?.(); };
  }, []);

  const ctxMenu = (s: Student) => (e: React.MouseEvent) => {
    e.preventDefault();
    const a = { getBoundingClientRect: () => ({ left: e.clientX, bottom: e.clientY, top: e.clientY, right: e.clientX }) } as unknown as HTMLElement;
    setMenu({ k: "ctx", a, s });
  };

  return (
    <>
      <div className="head">
        <div className="title"><h1>{settings?.room ? tr("Phòng {0}", settings.room) : tr("Lớp học")}<small>{settings?.teacher_name}</small></h1></div>
        <span className="chip ok"><span className="dot on" />{online.length}/{all.length}{" "}{tr("trực tuyến")}</span>
        <button className="chip" title={tr("Học sinh nhập mật khẩu này khi cài LabControl")} onClick={() => setShowPw(!showPw)}>
          <KeyRound size={13} />{tr("Mật khẩu phòng:")}{" "}<b className="mono">{showPw ? settings?.room_password : "••••••"}</b></button>
        {!settings?.accept_new && <span className="chip warn">{tr("Đang tắt nhận máy mới")}</span>}
        {showing && <button className="chip ok" onClick={() => api.stopShow()}><Cast size={13} />{tr("Đang trình chiếu · Dừng")}</button>}
        {policy.exam && <span className="chip warn"><ShieldAlert size={13} />{tr("Chế độ thi")}</span>}
        {policy.block_web.length > 0 && <span className="chip warn"><Globe size={13} />{tr("Chặn {0} trang web", policy.block_web.length)}</span>}
        {(policy.block_tools || policy.block_apps.length > 0) && !policy.exam && <span className="chip warn"><AppWindow size={13} />{tr("Chặn ứng dụng")}</span>}
        {policy.mute && <span className="chip warn"><VolumeX size={13} />{tr("Đã tắt tiếng")}</span>}
        {nOverride > 0 && <span className="chip" title={tr("Chuột phải vào máy › Dùng giới hạn chung của lớp")}><UserCog size={13} />{tr("{0} máy có giới hạn riêng", nOverride)}</span>}
        {outdated.length > 0 && <button className="chip warn" title={tr("Máy giáo viên đang dùng bản {0}", st.version)} onClick={() => askUpdate(outdated)}>
          <RefreshCw size={13} />{outdated.length}{" "}{tr("máy dùng bản cũ · Cập nhật")}</button>}
        <div className="right" />
      </div>

      <div className="toolbar">
        {tool(<Lock size={18} />, tr("Khóa máy"), openMenu("lock"),
              { on: anyLocked, tone: "err", caret: true, title: tr("Khóa hoặc mở khóa {0}", scopeLabel) })}
        {tool(<Cast size={18} />, tr("Trình chiếu"), openMenu("show"), { on: showing, caret: true })}
        {tool(<MessageSquare size={18} />, tr("Tin nhắn"), () => needOnline() && setDlg({ k: "message" }))}
        {tool(<FileUp size={18} />, tr("Gửi file"), openMenu("send"), { caret: true })}
        {tool(<Globe size={18} />, tr("Mở web"), () => needOnline() && setDlg({ k: "url" }))}
        {tool(<Rocket size={18} />, tr("Mở ứng dụng"), () => needOnline() && setDlg({ k: "run" }))}
        <div className="sep" />
        {tool(<BookOpen size={18} />, tr("Bài tập"), () => needOnline() && setDlg({ k: "assign" }))}
        {tool(<ClipboardCheck size={18} />, tr("Trắc nghiệm"), goTasks)}
        <div className="sep" />
        {tool(<Ban size={18} />, tr("Chặn web"), openMenu("web"), { on: curPol.block_web.length > 0, tone: "warn", caret: true })}
        {tool(<AppWindow size={18} />, tr("Chặn ứng dụng"), openMenu("apps"), { on: curPol.block_tools || curPol.block_apps.length > 0, tone: "warn", caret: true })}
        {tool(<X size={18} />, tr("Đóng ứng dụng"), () => needOnline() && setDlg({ k: "close" }))}
        {tool(<ShieldAlert size={18} />, tr("Chế độ thi"), () => setDlg({
          k: "confirm", title: curPol.exam ? tr("Kết thúc chế độ thi?") : scoped ? tr("Bật chế độ thi cho {0} máy đã chọn?", chosen.length) : tr("Bật chế độ thi cho cả lớp?"),
          text: curPol.exam ? tr("Máy học sinh được dùng lại trình duyệt và các ứng dụng.") :
            <>{tr("Máy học sinh sẽ hiện dải")}{" "}<b>{tr("“Đang trong giờ kiểm tra”")}</b>{tr("; trình duyệt, ứng dụng nhắn tin, điều khiển từ xa và các công cụ hệ thống (cmd, PowerShell, Task Manager...) bị đóng ngay khi mở. Các trang web đang chặn vẫn giữ nguyên.")}</>,
          ok: curPol.exam ? tr("Kết thúc") : tr("Bật chế độ thi"),
          run: () => setPol({ ...curPol, exam: !curPol.exam }, curPol.exam ? tr("Đã kết thúc chế độ thi") : tr("Đã bật chế độ thi")),
        }), { on: curPol.exam, tone: "warn" })}
        {tool(curPol.mute ? <VolumeX size={18} /> : <Volume2 size={18} />, curPol.mute ? tr("Bật tiếng") : tr("Tắt tiếng"),
              () => setPol({ ...curPol, mute: !curPol.mute }, curPol.mute ? tr("Đã bật lại tiếng") : tr("Đã tắt tiếng (học sinh không bật lại được)")),
              { on: !!curPol.mute, tone: "warn" })}
        <div className="sep" />
        {tool(<Power size={18} />, tr("Nguồn"), openMenu("power"), { caret: true })}
        {policyOn && tool(<ShieldOff size={18} />, tr("Bỏ giới hạn"), () => setPol(NO_POLICY, scoped ? tr("Đã bỏ giới hạn cho {0} máy", chosen.length) : tr("Đã bỏ mọi giới hạn")))}
        <span className="target-note">{targetNote}</span>
      </div>

      {st.update?.newer && !hideUpd && (
        <div className="banner info"><Download size={16} />{tr("Đã có LabControl")}{" "}<b>{st.update.version}</b>{" "}{tr("(bạn đang dùng")}{" "}{st.version}).
          <div className="row right">
            <button className="btn sm primary" onClick={() => api.openRelease(st.update!.url).catch((e) => toast(errText(e), "error"))}>{tr("Xem & tải về")}</button>
            <button className="btn sm ghost" onClick={() => setHideUpd(true)}>{tr("Ẩn")}</button>
          </div></div>
      )}
      {st.listenError && <div className="banner err"><WifiOff size={16} />{st.listenError}</div>}
      {pending.length > 0 && tab !== "pending" && (
        <div className="banner warn"><UserX size={16} />{pending.length}{" "}{tr("máy học sinh chưa vào được lớp (chưa ghép hoặc sai mật khẩu phòng).")}
          <button className="btn sm right" onClick={() => setTab("pending")}>Xem</button></div>
      )}

      <div className="filters">
        <input type="checkbox" className="check" checked={allSelected}
               onChange={() => allSelected ? setSel(new Set()) : selectAll()} title={tr("Chọn tất cả (Ctrl+A)")} />
        <div className="tabs">
          <button className={tab === "all" ? "active" : ""} onClick={() => setTab("all")}>{tr("Tất cả")}<b>{all.length}</b></button>
          <button className={tab === "online" ? "active" : ""} onClick={() => setTab("online")}>{tr("Trực tuyến")}<b>{online.length}</b></button>
          <button className={tab === "offline" ? "active" : ""} onClick={() => setTab("offline")}>{tr("Ngoại tuyến")}<b>{all.length - online.length}</b></button>
          <button className={tab === "pending" ? "active" : ""} onClick={() => setTab("pending")}>{tr("Chưa ghép")}<b>{pending.length}</b></button>
        </div>
        <div className="search"><Search size={15} /><input placeholder={tr("Tìm máy, học sinh, ứng dụng...")} value={q} onChange={(e) => setQ(e.target.value)} />
          {q && <X size={14} style={{ cursor: "pointer" }} onClick={() => setQ("")} />}</div>
        {sel.size > 0 && <button className="btn ghost sm" onClick={() => setSel(new Set())}>{tr("Bỏ chọn (")}{sel.size})</button>}
        <div className="row right">
          <select className="sel" value={sort} onChange={(e) => setSort(e.target.value as typeof sort)} title={tr("Sắp xếp")}>
            <option value="name">{tr("Theo tên máy")}</option><option value="user">{tr("Theo học sinh")}</option><option value="status">{tr("Trực tuyến trước")}</option>
          </select>
          {view !== "list" && <div className="seg" title={tr("Cỡ ảnh")}>
            {(["s", "m", "l"] as const).map((k) => <button key={k} className={size === k ? "active" : ""} onClick={() => setSize(k)}>
              {k === "s" ? "S" : k === "m" ? "M" : "L"}</button>)}
          </div>}
          <div className="seg">
            <button className={view === "grid" ? "active" : ""} onClick={() => setView("grid")} title={tr("Lưới màn hình")}><LayoutGrid size={16} /></button>
            <button className={view === "plan" ? "active" : ""} onClick={() => setView("plan")} title={tr("Sơ đồ phòng")}><Armchair size={16} /></button>
            <button className={view === "list" ? "active" : ""} onClick={() => setView("list")} title={tr("Danh sách")}><List size={16} /></button>
          </div>
        </div>
      </div>

      <div className="content" style={{ ["--tile" as string]: SIZES[size] }}>
        {tab === "pending" ? <PendingList /> : all.length === 0 ? <Setup openSettings={openSettings} /> :
          view === "grid" ? (
            list.length === 0 ? <Empty icon={<Search size={22} />} title={tr("Không có máy nào khớp")} /> :
              <div className="grid">{list.map((s) => <Tile key={s.id} s={s} thumb={thumbs.get(s.id)} sel={sel.has(s.id)}
                onClick={(e) => pick(s.id, e)} onCheck={() => toggle(s.id)} onOpen={() => s.state === "online" && setViewer(s)} onCtx={ctxMenu(s)} />)}</div>
          ) : view === "plan" ? (
            <SeatPlan list={list} sel={sel} thumbs={thumbs} toggle={toggle} pick={pick} open={(s) => s.state === "online" && setViewer(s)} ctx={ctxMenu} />
          ) : (
            <table className="table">
              <thead><tr><th style={{ width: 34 }} /><th>{tr("Máy")}</th><th>{tr("Học sinh")}</th><th>{tr("Trạng thái")}</th><th>{tr("Đang dùng")}</th><th>{tr("Địa chỉ")}</th><th>{tr("Phiên bản")}</th></tr></thead>
              <tbody>{list.map((s) => (
                <tr key={s.id} className={`click ${sel.has(s.id) ? "sel" : ""}`} onClick={(e) => pick(s.id, e)} onDoubleClick={() => s.state === "online" && setViewer(s)} onContextMenu={ctxMenu(s)}>
                  <td><input type="checkbox" className="check" checked={sel.has(s.id)} onChange={() => toggle(s.id)} onClick={(e) => e.stopPropagation()} /></td>
                  <td><b>{displayName(s)}</b>{s.label && <div className="small faint">{s.info.name}</div>}</td>
                  <td>{s.status?.user || s.info.user || "—"}</td>
                  <td><span className="row"><span className={`dot ${s.state === "online" ? "on" : ""}`} />{s.state === "online" ? tr("Trực tuyến") : tr("Ngoại tuyến")}
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
        <button onClick={() => { setMenu(null); if (needOnline()) lockNow(targets); }}><Lock size={16} />{tr("Khóa máy")}</button>
        <button onClick={() => { setMenu(null); if (needOnline()) unlockNow(targets); }}><Unlock size={16} />{tr("Mở khóa máy")}</button>
        <hr /><div className="mh">{targetNote}</div>
      </Menu>}
      {menu?.k === "send" && <Menu anchor={menu.a} onClose={() => setMenu(null)}>
        <button onClick={() => pickFiles(false)}><FileUp size={16} />{tr("Gửi file...")}</button>
        <button onClick={() => pickFiles(true)}><FolderUp size={16} />{tr("Gửi cả thư mục...")}</button>
        <hr /><div className="mh">{tr("Hoặc kéo file từ máy tính thả vào cửa sổ này")}</div>
        <div className="mh" style={{ textTransform: "none", letterSpacing: 0, paddingTop: 0 }}>{targetNote}{tr(". Học sinh nhận ở thư mục TuGiaoVien trên màn hình nền.")}</div>
      </Menu>}
      {menu?.k === "show" && <Menu anchor={menu.a} onClose={() => setMenu(null)}>
        <div className="mh">{tr("Chiếu màn hình của bạn cho học sinh")}</div>
        <button onClick={() => { setMenu(null); if (needOnline()) run(tr("Đang trình chiếu toàn màn hình"), () => api.startShow(targets, true)); }}><Cast size={16} />{tr("Toàn màn hình (khóa máy học sinh)")}</button>
        <button onClick={() => { setMenu(null); if (needOnline()) run(tr("Đang trình chiếu trong cửa sổ"), () => api.startShow(targets, false)); }}><AppWindow size={16} />{tr("Trong cửa sổ")}</button>
        {showing && <><hr /><button className="danger" onClick={() => { setMenu(null); api.stopShow(); }}><MonitorOff size={16} />{tr("Dừng trình chiếu")}</button></>}
      </Menu>}
      {menu?.k === "web" && <WebMenu anchor={menu.a} onClose={() => setMenu(null)} policy={curPol} scope={scopeLabel} list={settings?.web_list ?? []}
                                     apply={(p) => setPol(p, p.block_web.length ? tr("Đang chặn {0} trang web", p.block_web.length) : tr("Đã bỏ chặn web"))}
                                     edit={() => { setMenu(null); openSettings("lists"); }} />}
      {menu?.k === "apps" && <AppsMenu anchor={menu.a} onClose={() => setMenu(null)} policy={curPol} scope={scopeLabel} list={settings?.app_list ?? []}
                                       apply={(p) => setPol(p, tr("Đã cập nhật chặn ứng dụng"))} edit={() => { setMenu(null); openSettings("lists"); }} />}
      {menu?.k === "power" && <Menu anchor={menu.a} onClose={() => setMenu(null)}>
        {([["shutdown", tr("Tắt máy")], ["restart", tr("Khởi động lại")], ["logoff", tr("Đăng xuất")]] as const).map(([a, l]) => (
          <button key={a} onClick={() => { setMenu(null); if (needOnline()) setDlg({ k: "confirm", title: `${l}?`, danger: true, ok: l,
            text: tr("{0} {1} máy học sinh. Bài đang làm chưa lưu có thể bị mất.", l, targets.length), run: () => api.send(targets, { t: "power", action: a }) }); }}>
            <Power size={16} />{l}</button>))}
        <hr /><div className="mh">{targetNote}</div>
      </Menu>}
      {menu?.k === "ctx" && menu.s && (() => {
        const s = menu.s; const id = [s.id]; const on = s.state === "online";
        return <Menu anchor={menu.a} onClose={() => setMenu(null)}>
          <div className="mh">{displayName(s)}</div>
          {on && <>
            <button onClick={() => { setMenu(null); setViewer(s); }}><Eye size={16} />{tr("Xem / điều khiển")}</button>
            {s.status?.locked
              ? <button onClick={() => { setMenu(null); unlockNow(id, displayName(s)); }}><Unlock size={16} />{tr("Mở khóa")}</button>
              : <button onClick={() => { setMenu(null); lockNow(id, displayName(s)); }}><Lock size={16} />{tr("Khóa máy")}</button>}
            <button onClick={() => { setMenu(null); setSel(new Set(id)); setDlg({ k: "message" }); }}><MessageSquare size={16} />{tr("Gửi tin nhắn")}</button>
            <button onClick={() => { setMenu(null); setSel(new Set(id)); setDlg({ k: "url" }); }}><Globe size={16} />{tr("Mở trang web…")}</button>
            <button onClick={() => { setMenu(null); setSel(new Set(id)); setDlg({ k: "run" }); }}><Rocket size={16} />{tr("Mở ứng dụng…")}</button>
          </>}
          {on && s.info.version !== st.version && <button onClick={() => { setMenu(null); askUpdate([s]); }}><RefreshCw size={16} />{tr("Cập nhật phần mềm (")}{s.info.version || "?"} → {st.version})</button>}
          {on && (() => {
            // bỏ chặn riêng từng thứ cho máy này (các máy khác giữ nguyên)
            const p = { ...NO_POLICY, ...effOf(s.id) };
            const one = (np: Policy, label: string) => { setMenu(null); run(label, () => api.setPolicy(np, [s.id])); };
            const n = displayName(s);
            const items = [
              p.block_web.length > 0 && <button key="w" onClick={() => one({ ...p, block_web: [] }, tr("Đã mở web cho {0}", n))}><Globe size={16} />{tr("Mở web cho máy này")}</button>,
              (p.block_apps.length > 0 || p.block_tools) && !p.exam && <button key="a" onClick={() => one({ ...p, block_apps: [], block_tools: false }, tr("Đã bỏ chặn ứng dụng cho {0}", n))}><AppWindow size={16} />{tr("Bỏ chặn ứng dụng cho máy này")}</button>,
              p.mute && <button key="m" onClick={() => one({ ...p, mute: false }, tr("Đã bật tiếng cho {0}", n))}><Volume2 size={16} />{tr("Bật tiếng cho máy này")}</button>,
              p.exam && <button key="e" onClick={() => one({ ...p, exam: false }, tr("Đã tắt chế độ thi cho {0}", n))}><ShieldAlert size={16} />{tr("Tắt chế độ thi cho máy này")}</button>,
            ].filter(Boolean);
            return items.length ? <><hr />{items}</> : null;
          })()}
          {on && !isFree(effOf(s.id)) && <button onClick={() => { setMenu(null); run(tr("Đã bỏ giới hạn cho {0}", displayName(s)), () => api.setPolicy(NO_POLICY, [s.id])); }}>
            <ShieldOff size={16} />{tr("Bỏ mọi giới hạn cho máy này")}</button>}
          {on && overrides[s.id] && <button onClick={() => { setMenu(null); run(tr("{0} dùng lại giới hạn chung", displayName(s)), () => api.clearOverride([s.id])); }}>
            <ShieldCheck size={16} />{tr("Dùng giới hạn chung của lớp")}</button>}
          <button onClick={() => { setMenu(null); setDlg({ k: "rename", s }); }}><Pencil size={16} />{tr("Đặt tên hiển thị")}</button>
          {!on && <button className="danger" onClick={() => { setMenu(null); setDlg({ k: "confirm", title: tr("Gỡ máy khỏi danh sách?"), danger: true, ok: tr("Gỡ"),
            text: tr("Máy này sẽ hiện lại khi tự nối về. Muốn máy không nối về nữa, hãy gỡ LabControl trên máy đó."), run: () => api.remove(s.id) }); }}>
            <Trash2 size={16} />{tr("Gỡ khỏi danh sách")}</button>}
        </Menu>;
      })()}

      {/* ------------------------------------------------ hộp thoại */}
      {dlg?.k === "message" && <TextDialog title={tr("Gửi tin nhắn")} label={tr("Nội dung")} note={targetNote} multiline ok={tr("Gửi")}
        onClose={() => setDlg(null)} onOk={(t) => run(tr("Đã gửi tin nhắn"), () => api.send(targets, { t: "message", text: t }))} />}
      {dlg?.k === "url" && <TextDialog title={tr("Mở trang web trên máy học sinh")} label={tr("Địa chỉ")} initial="https://" note={targetNote} ok={tr("Mở")}
        onClose={() => setDlg(null)} onOk={(t) => {
          const url = /^https?:\/\//.test(t) ? t : `https://${t}`;
          return run(tr("Đã mở trang web"), () => api.send(targets, { t: "open_url", url }));
        }} />}
      {dlg?.k === "run" && <RunAppDialog note={targetNote} custom={settings?.run_list ?? []}
        oses={online.filter((s) => targets.includes(s.id)).map((s) => s.info.os)}
        onClose={() => setDlg(null)} onEdit={() => { setDlg(null); openSettings("lists"); }}
        onOk={(c) => run(tr("Đã mở {0}", c), () => api.send(targets, { t: "run_app", cmd: c }))} />}
      {dlg?.k === "close" && <CloseAppDialog note={targetNote} apps={[...new Set(online.filter((s) => targets.includes(s.id)).map((s) => s.status?.app).filter(Boolean) as string[])]}
        onClose={() => setDlg(null)} onOk={(n) => run(tr("Đã yêu cầu đóng {0}", n), () => api.send(targets, { t: "close_app", name: n }))} />}
      {dlg?.k === "assign" && <AssignDialog note={targetNote} onClose={() => setDlg(null)}
        onOk={async (title, text) => { await api.assign(targets, title, text); toast(tr("Đã giao bài “{0}” cho {1} máy", title, targets.length)); }} />}
      {dlg?.k === "rename" && <TextDialog title={tr("Tên hiển thị – {0}", dlg.s.info.name)} label={tr("Tên (vd: tên học sinh ngồi máy này)")} initial={dlg.s.label}
        ok={tr("Lưu")} allowEmpty onClose={() => setDlg(null)} onOk={(t) => api.setLabel(dlg.s.id, t)} />}
      {dlg?.k === "confirm" && <Confirm title={dlg.title} message={dlg.text} danger={dlg.danger} confirmText={dlg.ok}
        onClose={() => setDlg(null)} onConfirm={async () => { await dlg.run(); }} />}

      {viewer && <Viewer s={viewer} onClose={() => setViewer(null)} />}
      {dropOver && <div className="drop-zone"><div><FileUp size={24} />{tr("Thả để gửi cho")}{" "}{chosen.length ? tr("{0} máy đã chọn", chosen.length) : tr("cả lớp ({0} máy)", online.length)}</div></div>}
    </>
  );
}

// ------------------------------------------------------------------ ô màn hình
function Flags({ s }: { s: Student }) {
  const { policy, overrides } = useStore();
  const t = s.status;
  if (!t || s.state !== "online") return null;
  return <>
    {t.locked && <span className="flag err" title={tr("Đang khóa")}><Lock size={12} /></span>}
    {t.viewed && <span className="flag err" title={tr("Đang được xem")}><Eye size={12} /></span>}
    {t.showing && <span className="flag pri" title={tr("Đang xem trình chiếu")}><Cast size={12} /></span>}
    {t.exam && <span className="flag warn" title={tr("Chế độ thi")}><ShieldAlert size={12} /></span>}
    {t.muted && <span className="flag warn" title={tr("Đã tắt tiếng")}><VolumeX size={12} /></span>}
    {t.web_blocked && !t.exam && <span className="flag warn" title={tr("Đang chặn web")}><Ban size={12} /></span>}
    {overrides[s.id] && <span className="flag pri" title={tr("Giới hạn riêng cho máy này")}><UserCog size={12} /></span>}
    {!t.helper && (overrides[s.id] ?? policy).block_web.length > 0 && <span className="flag" title={tr("Máy chưa cài dịch vụ Helper: không chặn web được")}><ShieldOff size={12} /></span>}
  </>;
}

function Tile({ s, thumb, sel, onClick, onCheck, onOpen, onCtx, onGrab }: {
  s: Student; thumb?: string; sel: boolean; onClick: (e: React.MouseEvent) => void; onCheck: () => void; onOpen: () => void; onCtx: (e: React.MouseEvent) => void;
  onGrab?: (e: React.PointerEvent) => void;
}) {
  const on = s.state === "online";
  const user = s.status?.user || s.info.user;
  return (
    <div className={`tile ${sel ? "sel" : ""} ${on ? "" : "off"}`} onClick={onClick} onDoubleClick={onOpen} onContextMenu={onCtx}
         onPointerDown={onGrab}>
      <div className="shot">
        {on && thumb ? <img src={thumb} alt="" draggable={false} /> : <div className="ph">{on ? <div className="spinner" /> : <MonitorOff size={26} />}</div>}
        <div className="flags"><Flags s={s} /></div>
        <input type="checkbox" className="check ck" checked={sel} onChange={onCheck} onClick={(e) => e.stopPropagation()} />
      </div>
      <div className="meta">
        <span className={`dot ${on ? "on" : ""}`} />
        <div style={{ minWidth: 0 }}>
          <div className="nm">{displayName(s)}</div>
          <div className="us">{user ? user : on ? tr("Chưa đăng nhập") : tr("Ngoại tuyến")}</div>
        </div>
        {on && <button className="btn ghost sm right" title={tr("Xem / điều khiển")} onClick={(e) => { e.stopPropagation(); onOpen(); }}><MousePointer2 size={14} /></button>}
      </div>
      {on && s.status?.app && <div className="cur-app" title={s.status.title}>{s.status.app}</div>}
    </div>
  );
}

// ------------------------------------------------------------------ sơ đồ phòng
function SeatPlan({ list, sel, thumbs, toggle, pick, open, ctx }: {
  list: Student[]; sel: Set<string>; thumbs: Map<string, string>; toggle: (id: string) => void;
  pick: (id: string, e: React.MouseEvent) => void; open: (s: Student) => void;
  ctx: (s: Student) => (e: React.MouseEvent) => void;
}) {
  const { settings } = useStore();
  const cols = Math.max(2, settings?.seat_cols || 6);
  const [over, setOver] = useState<string | null>(null);
  const [drag, setDrag] = useState<{ id: string; name: string; x: number; y: number } | null>(null);
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
  // Kéo thả bằng con trỏ (không dùng HTML5 drag & drop, vốn bị tắt trong cửa sổ ứng dụng trên Windows)
  const target = (x: number, y: number) =>
    (document.elementFromPoint(x, y) as HTMLElement | null)?.closest<HTMLElement>("[data-drop]")?.dataset.drop ?? null;
  const grab = (s: Student) => (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    const x0 = e.clientX, y0 = e.clientY;
    let moved = false;
    const move = (ev: PointerEvent) => {
      if (!moved && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 6) return;
      moved = true;
      setDrag({ id: s.id, name: displayName(s), x: ev.clientX, y: ev.clientY });
      setOver(target(ev.clientX, ev.clientY));
    };
    const up = (ev: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setDrag(null); setOver(null);
      if (!moved) return;
      // chặn cú "click" ngay sau khi thả để không chọn/bỏ chọn máy
      const stop = (ce: Event) => { ce.stopPropagation(); ce.preventDefault(); };
      window.addEventListener("click", stop, { capture: true, once: true });
      setTimeout(() => window.removeEventListener("click", stop, { capture: true }), 50);
      const t = target(ev.clientX, ev.clientY);
      if (t === "tray") api.setSeat(s.id, null);
      else if (t) { const [c, r] = t.split(",").map(Number); api.setSeat(s.id, [c, r]); }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  return (
    <div className="plan" style={drag ? { cursor: "grabbing", userSelect: "none" } : undefined}>
      <div className="row">
        <span className="small muted">{tr("Kéo thả máy vào chỗ ngồi giống phòng thật. Thả lên máy khác để đổi chỗ.")}</span>
        <div className="row right">
          <span className="small">{tr("Số cột")}</span>
          <button className="btn sm" onClick={() => setCols(cols - 1)}>−</button><b>{cols}</b>
          <button className="btn sm" onClick={() => setCols(cols + 1)}>+</button>
          <button className="btn sm" onClick={() => api.autoSeats(cols)}><Wand2 size={14} />{tr("Xếp tự động theo tên")}</button>
        </div>
      </div>
      <div className="desk">{tr("Bàn giáo viên")}</div>
      <div className="seats" style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {Array.from({ length: rows * cols }, (_, i) => {
          const c = i % cols, r = Math.floor(i / cols), k = `${c},${r}`;
          const s = placed.get(k);
          return <div key={k} data-drop={k} className={`seat ${over === k ? "over" : ""}`}>
            {s && <Tile s={s} thumb={thumbs.get(s.id)} sel={sel.has(s.id)} onClick={(e) => pick(s.id, e)} onCheck={() => toggle(s.id)} onOpen={() => open(s)} onCtx={ctx(s)} onGrab={grab(s)} />}
          </div>;
        })}
      </div>
      <div data-drop="tray" className={`tray ${over === "tray" ? "over" : ""}`}>
        <div className="small muted" style={{ marginBottom: 8 }}>{tr("Chưa xếp chỗ (")}{unplaced.length}{tr(") – kéo lên sơ đồ")}</div>
        {unplaced.length ? <div className="grid">{unplaced.map((s) => <Tile key={s.id} s={s} thumb={thumbs.get(s.id)} sel={sel.has(s.id)}
          onClick={(e) => pick(s.id, e)} onCheck={() => toggle(s.id)} onOpen={() => open(s)} onCtx={ctx(s)} onGrab={grab(s)} />)}</div> : <div className="small faint">{tr("Mọi máy đã có chỗ.")}</div>}
      </div>
      {drag && <div className="drag-ghost" style={{ left: drag.x + 12, top: drag.y + 12 }}>{drag.name}</div>}
    </div>
  );
}

// ------------------------------------------------------------------ máy chưa vào được lớp
function PendingList() {
  const { pending, settings } = useStore();
  if (!pending.length) return <Empty icon={<ShieldCheck size={22} />} title={tr("Không có máy nào bị từ chối")}>
    <div className="small muted">{tr("Máy nhập sai mật khẩu phòng hoặc chưa được ghép sẽ hiện ở đây.")}</div></Empty>;
  return <div className="col" style={{ gap: 12 }}>
    <div className="banner info" style={{ margin: 0 }}>{tr("Máy dưới đây đã tìm thấy máy giáo viên nhưng chưa vào được lớp. Hãy kiểm tra mật khẩu phòng trên máy học sinh khớp với")}{" "}<b className="mono">&nbsp;{settings?.room_password}&nbsp;</b>
      {!settings?.accept_new && <>{" "}{tr("và bật “Nhận máy mới” trong Cài đặt")}</>}.</div>
    <table className="table">
      <thead><tr><th>{tr("Máy")}</th><th>{tr("Người dùng")}</th><th>{tr("Địa chỉ")}</th><th>{tr("Lý do")}</th><th>{tr("Lúc")}</th><th /></tr></thead>
      <tbody>{pending.map((p) => <tr key={p.id}>
        <td><b>{p.name}</b></td><td>{p.user || "—"}</td><td className="mono">{p.ip}</td>
        <td className="small" style={{ color: "var(--warn)" }}>{p.reason}</td><td className="small faint">{time(p.at)}</td>
        <td><button className="btn ghost sm" onClick={() => api.dismissPending(p.id)}>{tr("Ẩn")}</button></td></tr>)}</tbody>
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
    <div className="card-h"><h2>{tr("Đưa máy học sinh vào lớp")}</h2></div>
    <div className="card-b">
      <div className="steps">
        <div className="step"><div className="col">
          <b>{tr("Máy giáo viên đã sẵn sàng")}</b>
          <span className="small muted">{tr("Máy học sinh trong cùng mạng LAN sẽ tự tìm thấy máy này")}{addresses.length ? <>{" "}{tr("(địa chỉ")}{" "}<span className="mono">{addresses.join(", ")}</span>)</> : null}{tr(". Nếu Windows hỏi cho phép LabControl qua tường lửa, hãy chọn")}{" "}<b>{tr("Cho phép")}</b>.</span>
        </div></div>
        <div className="step"><div className="col">
          <b>{tr("Mật khẩu phòng:")}{" "}<span className="pw">{pw}</span></b>
          <span className="small muted">{tr("Học sinh/kỹ thuật viên nhập mật khẩu này một lần khi cài. Đổi trong")}{" "}<a href="#" onClick={(e) => { e.preventDefault(); openSettings("room"); }}>{tr("Cài đặt")}</a>.</span>
        </div></div>
        <div className="step"><div className="col" style={{ minWidth: 0 }}>
          <b>{tr("Cài LabControl trên từng máy học sinh (quyền quản trị)")}</b>
          <span className="small muted">{tr("Chạy")}{" "}<span className="mono">cai-may-hoc-sinh.cmd</span>{" "}{tr("và nhập mật khẩu phòng, hoặc dùng lệnh:")}</span>
          <div className="code-box">{cmd}</div>
          <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => navigator.clipboard.writeText(cmd).then(() => toast(tr("Đã chép lệnh")))}>{tr("Chép lệnh")}</button>
        </div></div>
        <div className="step"><div className="col">
          <b>{tr("Học sinh đăng nhập – máy tự hiện ở đây")}</b>
          <span className="small muted">{tr("Lần đầu máy ghép bằng mật khẩu phòng, các lần sau tự vào lớp. Máy học sinh không cần mở tường lửa.")}</span>
        </div></div>
      </div>
    </div>
  </div>;
}

// ------------------------------------------------------------------ menu chặn
function WebMenu({ anchor, onClose, policy, scope, list, apply, edit }: {
  anchor: HTMLElement; scope: string; onClose: () => void; policy: Policy; list: string[]; apply: (p: Policy) => void; edit: () => void;
}) {
  const [on, setOn] = useState<Set<string>>(new Set(policy.block_web));
  const [extra, setExtra] = useState("");
  const all = [...new Set([...list, ...policy.block_web])];
  const add = () => {
    const d = extra.trim().replace(/^https?:\/\//, "").split("/")[0].toLowerCase();
    if (d) { setOn((s) => new Set(s).add(d)); setExtra(""); }
  };
  return <Menu anchor={anchor} onClose={onClose} width={340}>
    <div className="mh">{tr("Chặn trang web")} – {scope}</div>
    <div style={{ padding: "4px 10px 8px" }} className="col">
      <div className="pills">{[...new Set([...all, ...on])].map((d) => <button key={d} className={`pill ${on.has(d) ? "on" : ""}`}
        onClick={() => setOn((s) => { const n = new Set(s); if (n.has(d)) n.delete(d); else n.add(d); return n; })}>{d}</button>)}</div>
      <div className="row"><input className="input" placeholder={tr("Thêm tên miền, vd: game.vn")} value={extra} onChange={(e) => setExtra(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && add()} /><button className="btn sm" onClick={add}>{tr("Thêm")}</button></div>
      <div className="small faint">{tr("Chặn cả www. và m. của tên miền. Cần dịch vụ Helper trên máy học sinh (cài bằng lệnh install).")}</div>
      <div className="row">
        <button className="btn sm ghost" onClick={edit}>{tr("Sửa danh sách...")}</button>
        <button className="btn sm right" onClick={() => { apply({ ...policy, block_web: [] }); onClose(); }}>{tr("Bỏ chặn")}</button>
        <button className="btn sm primary" onClick={() => { apply({ ...policy, block_web: [...on] }); onClose(); }}>{tr("Chặn {0} trang", on.size)}</button>
      </div>
    </div>
  </Menu>;
}

function AppsMenu({ anchor, onClose, policy, scope, list, apply, edit }: {
  anchor: HTMLElement; scope: string; onClose: () => void; policy: Policy; list: string[]; apply: (p: Policy) => void; edit: () => void;
}) {
  const [tools, setTools] = useState(policy.block_tools);
  const [on, setOn] = useState<Set<string>>(new Set(policy.block_apps));
  const all = [...new Set([...list, ...policy.block_apps])];
  return <Menu anchor={anchor} onClose={onClose} width={340}>
    <div className="mh">{tr("Chặn ứng dụng")} – {scope}</div>
    <div style={{ padding: "4px 10px 8px" }} className="col">
      <Switch checked={tools} onChange={setTools} label={tr("Chặn công cụ hệ thống")} hint="cmd, PowerShell, Task Manager, Registry, Terminal..." />
      <div className="small muted" style={{ marginTop: 6 }}>{tr("Đóng ngay khi mở:")}</div>
      <div className="pills">{all.map((d) => <button key={d} className={`pill ${on.has(d) ? "on" : ""}`}
        onClick={() => setOn((s) => { const n = new Set(s); if (n.has(d)) n.delete(d); else n.add(d); return n; })}>{d}</button>)}</div>
      <div className="row">
        <button className="btn sm ghost" onClick={edit}>{tr("Sửa danh sách...")}</button>
        <button className="btn sm primary right" onClick={() => { apply({ ...policy, block_tools: tools, block_apps: [...on] }); onClose(); }}>{tr("Áp dụng")}</button>
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
  return <Modal title={title} onClose={onClose} footer={<><button className="btn" onClick={onClose}>{tr("Hủy")}</button>
    <button className="btn primary" disabled={!t.trim() && !allowEmpty} onClick={go}>{ok}</button></>}>
    <div className="field"><label>{label}</label>
      {multiline ? <textarea ref={ref} autoFocus className="input" rows={4} value={t} onChange={(e) => setT(e.target.value)}
                             onKeyDown={(e) => e.key === "Enter" && e.ctrlKey && go()} />
        : <input ref={ref} autoFocus className="input" value={t} onChange={(e) => setT(e.target.value)} onKeyDown={(e) => e.key === "Enter" && go()} />}
      {note && <span className="hint">{note}{multiline ? tr(" · Ctrl+Enter để gửi") : ""}</span>}</div>
  </Modal>;
}

function CloseAppDialog({ note, apps, onClose, onOk }: { note: string; apps: string[]; onClose: () => void; onOk: (n: string) => unknown }) {
  const [n, setN] = useState("");
  return <Modal title={tr("Đóng ứng dụng trên máy học sinh")} onClose={onClose} footer={<><button className="btn" onClick={onClose}>{tr("Hủy")}</button>
    <button className="btn primary" disabled={!n.trim()} onClick={async () => { await onOk(n.trim()); onClose(); }}>{tr("Đóng ứng dụng")}</button></>}>
    <div className="field"><label>{tr("Tên tiến trình")}</label>
      <input className="input" autoFocus placeholder="vd: chrome.exe, zalo.exe, game.exe" value={n} onChange={(e) => setN(e.target.value)} />
      <span className="hint">{note}{tr(". Muốn chặn hẳn (mở lại cũng bị đóng), dùng “Chặn ứng dụng”.")}</span></div>
    {apps.length > 0 && <div className="field"><label>{tr("Đang mở trên các máy")}</label>
      <div className="pills">{apps.map((a) => <button key={a} className={`pill ${n === a ? "on" : ""}`} onClick={() => setN(a)}>{a}</button>)}</div></div>}
  </Modal>;
}

function AssignDialog({ note, onClose, onOk }: { note: string; onClose: () => void; onOk: (title: string, text: string) => Promise<void> }) {
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [err, setErr] = useState("");
  return <Modal title={tr("Giao bài tập")} onClose={onClose} footer={<><button className="btn" onClick={onClose}>{tr("Hủy")}</button>
    <button className="btn primary" disabled={!title.trim()} onClick={async () => {
      try { await onOk(title.trim(), text); onClose(); } catch (e) { setErr(errText(e)); }
    }}>{tr("Giao bài")}</button></>}>
    <div className="field"><label>{tr("Tên bài")}</label><input className="input" autoFocus placeholder={tr("vd: Bài 3 – Vòng lặp")} value={title} onChange={(e) => setTitle(e.target.value)} /></div>
    <div className="field"><label>{tr("Đề bài / hướng dẫn")}</label><textarea className="input" rows={5} value={text} onChange={(e) => setText(e.target.value)} />
      <span className="hint">{tr("Máy học sinh tạo thư mục")}{" "}<b>BaiLam\{title.trim() || tr("<tên bài>")}</b>{" "}{tr("trên màn hình nền, kèm file DE-BAI.txt.")}{" "}{note}{tr(". Thu bài ở trang “Bài & kiểm tra”.")}</span></div>
    {err && <div className="err">{err}</div>}
  </Modal>;
}

// ------------------------------------------------------------------ mở ứng dụng
const WIN_APPS = (): [string, string][] => [
  ["Notepad", "notepad"], ["Paint", "mspaint"], [tr("Máy tính"), "calc"], ["Word", "winword"], ["Excel", "excel"],
  ["PowerPoint", "powerpnt"], ["File Explorer", "explorer"], ["Chrome", "chrome"], ["Edge", "msedge"],
];
const LINUX_APPS = (): [string, string][] => [
  [tr("Soạn văn bản"), "xed"], ["Writer", "libreoffice --writer"], ["Calc", "libreoffice --calc"],
  ["Impress", "libreoffice --impress"], [tr("Quản lý file"), "nemo"], ["Firefox", "firefox"], ["Chrome", "google-chrome"],
];

function RunAppDialog({ note, custom, oses, onClose, onOk, onEdit }: {
  note: string; custom: string[]; oses: string[]; onClose: () => void; onOk: (c: string) => unknown; onEdit: () => void;
}) {
  const [c, setC] = useState("");
  const win = oses.some((o) => /windows/i.test(o));
  const lin = oses.some((o) => !/windows/i.test(o));
  const quick: [string, string][] = [
    ...custom.map((x) => [x.split(/[\\/]/).pop()!.replace(/\.exe"?$/i, "").replace(/^"/, ""), x] as [string, string]),
    ...(win ? WIN_APPS() : []), ...(lin ? LINUX_APPS() : []),
  ];
  const go = async () => { if (c.trim()) { await onOk(c.trim()); onClose(); } };
  return <Modal title={tr("Mở ứng dụng trên máy học sinh")} onClose={onClose} footer={<>
    <button className="btn ghost" style={{ marginRight: "auto" }} onClick={onEdit}>{tr("Sửa danh sách...")}</button>
    <button className="btn" onClick={onClose}>{tr("Hủy")}</button>
    <button className="btn primary" disabled={!c.trim()} onClick={go}>{tr("Mở")}</button></>}>
    <div className="field"><label>{tr("Ứng dụng")}</label>
      <input className="input mono" autoFocus placeholder={win ? "vd: notepad, winword, C:\\...\\Scratch 3.exe" : "vd: firefox, libreoffice --writer"}
             value={c} onChange={(e) => setC(e.target.value)} onKeyDown={(e) => e.key === "Enter" && go()} />
      <span className="hint">{tr("Tên chương trình, đường dẫn đầy đủ, hoặc lệnh có tham số.")}{" "}{note}.</span></div>
    {quick.length > 0 && <div className="field"><label>{tr("Chọn nhanh")}</label>
      <div className="pills">{quick.map(([label, cmd]) => <button key={cmd} className={`pill ${c === cmd ? "on" : ""}`} title={cmd} onClick={() => setC(cmd)}>{label}</button>)}</div></div>}
  </Modal>;
}
