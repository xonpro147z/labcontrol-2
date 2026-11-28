import { tr } from "../i18n";
import { useState, type ReactNode } from "react";
import { Ban, Download, Film, Gauge, ImagePlus, Info, KeyRound, Laptop, Lock, Palette, RefreshCw, RotateCcw, Send, Trash2 } from "lucide-react";
import { LANGS, lang } from "../i18n";
import { ACCENTS, WALLPAPERS } from "../theme";
import { open } from "@tauri-apps/plugin-dialog";
import { api, reload, setSettingsLocal, useStore, type LockMedia, type Settings } from "../api";
import { SOURCE_URL, VERSION } from "../components/About";
import { Modal, Switch, errText, toast } from "../components/ui";

const SECTIONS = (): { k: string; label: string; icon: ReactNode }[] => [
  { k: "look", label: tr("Giao diện & ngôn ngữ"), icon: <Palette size={16} /> },
  { k: "room", label: tr("Phòng & ghép máy"), icon: <KeyRound size={16} /> },
  { k: "install", label: tr("Cài máy học sinh"), icon: <Laptop size={16} /> },
  { k: "lists", label: tr("Danh sách chặn"), icon: <Ban size={16} /> },
  { k: "lock", label: tr("Màn hình khóa"), icon: <Lock size={16} /> },
  { k: "perf", label: tr("Hình ảnh & tốc độ"), icon: <Gauge size={16} /> },
  { k: "about", label: tr("Giới thiệu"), icon: <Info size={16} /> },
];

function randomPw() {
  const a = new Uint32Array(1);
  crypto.getRandomValues(a);
  return String(100000 + (a[0] % 900000));
}

export function SettingsModal({ initial, onClose }: { initial: string; onClose: () => void }) {
  const st = useStore();
  const [sec, setSec] = useState(initial);
  const [orig] = useState<Settings>(st.settings!);
  const [s, setS] = useState<Settings>(st.settings!);
  const [webText, setWebText] = useState(st.settings!.web_list.join("\n"));
  const [appText, setAppText] = useState(st.settings!.app_list.join("\n"));
  const [runText, setRunText] = useState((st.settings!.run_list ?? []).join("\n"));
  const [err, setErr] = useState("");
  const pwChanged = s.room_password.trim() !== st.settings!.room_password;
  const up = (p: Partial<Settings>) => setS({ ...s, ...p });
  // đổi giao diện / ngôn ngữ: xem trước ngay; bấm Hủy thì trả lại như cũ
  const look = (p: Partial<Settings>) => { const n = { ...s, ...p }; setS(n); setSettingsLocal({ ...orig, ...pickLook(n) }); };
  // ảnh nền màn hình khóa được lưu ngay khi chọn (không theo nút Hủy)
  const cancel = () => { setSettingsLocal({ ...orig, lock_media: s.lock_media }); onClose(); };
  const setMedia = (m: LockMedia) => { setS((x) => ({ ...x, lock_media: m })); setSettingsLocal({ ...st.settings!, lock_media: m }); };

  const save = async () => {
    const next: Settings = {
      ...s, room_password: s.room_password.trim(), room: s.room.trim(),
      web_list: webText.split(/[\s,]+/).map((x) => x.trim().toLowerCase()).filter(Boolean),
      app_list: appText.split(/[\n,]+/).map((x) => x.trim()).filter(Boolean),
      run_list: runText.split("\n").map((x) => x.trim()).filter(Boolean),
    };
    try {
      await api.setSettings(next);
      setSettingsLocal(next);
      if (pwChanged && st.students.size) {
        const n = await api.pushPassword();
        toast(tr("Đã lưu. Mật khẩu phòng mới đã gửi tới {0} máy đang trực tuyến", n));
      } else toast(tr("Đã lưu cài đặt"));
      await reload();
      onClose();
    } catch (e) { setErr(errText(e)); }
  };

  const room = s.room.trim() ? ` --room "${s.room.trim()}"` : "";
  const winCmd = `labcontrol-agent.exe install --password "${s.room_password.trim()}"${room}`;
  const linCmd = `sudo ./labcontrol-agent install --password "${s.room_password.trim()}"${room}`;
  const ipCmd = `${winCmd} --teacher-ip ${st.addresses[0] ?? tr("<IP máy giáo viên>")}`;
  const [checking, setChecking] = useState(false);
  const online = [...st.students.values()].filter((x) => x.state === "online");
  const outdated = online.filter((x) => x.info.version && x.info.version !== st.version);
  const push = async (file: string | null) => {
    try {
      const n = await api.pushUpdate((file ? online : outdated).map((x) => x.id), file);
      toast(n ? tr("Đang gửi bản cập nhật tới {0} máy", n) : tr("Không có máy nào cần cập nhật"));
    } catch (e) { toast(errText(e), "error"); }
  };
  const pickFile = async () => {
    const f = await open({ multiple: false, title: tr("Chọn file labcontrol-agent (.exe cho Windows, không đuôi cho Linux)") });
    if (f) push(String(f));
  };
  const copy = (t: string) => navigator.clipboard.writeText(t).then(() => toast(tr("Đã chép")));

  return (
    <Modal title={tr("Cài đặt")} onClose={cancel} wide bare footer={<>
      {err && <span className="err" style={{ marginRight: "auto" }}>{err}</span>}
      <button className="btn" onClick={cancel}>{tr("Hủy")}</button><button className="btn primary" onClick={save}>{tr("Lưu")}</button></>}>
      <div className="settings">
        <nav>{SECTIONS().map((x) => <button key={x.k} className={sec === x.k ? "active" : ""} onClick={() => setSec(x.k)}>{x.icon}{x.label}</button>)}</nav>
        <section>
          {sec === "look" && <LookSection s={s} look={look} />}
          {sec === "room" && <>
            <div className="field"><label>{tr("Tên giáo viên")}</label><input className="input" value={s.teacher_name} onChange={(e) => up({ teacher_name: e.target.value })} /></div>
            <div className="field"><label>{tr("Tên phòng máy")}</label><input className="input" placeholder={tr("vd: P.Tin 1 (để trống nếu mạng chỉ có một phòng)")} value={s.room} onChange={(e) => up({ room: e.target.value })} />
              <span className="hint">{tr("Khi nhiều phòng dùng chung mạng, đặt tên phòng để máy học sinh chỉ vào đúng phòng của mình. Tên phòng phải giống lúc cài máy học sinh.")}</span></div>
            <div className="field"><label>{tr("Mật khẩu phòng")}</label>
              <div className="row"><input className="input mono" style={{ fontSize: 16, letterSpacing: ".08em" }} value={s.room_password} onChange={(e) => up({ room_password: e.target.value })} />
                <button className="btn" onClick={() => up({ room_password: randomPw() })}><RefreshCw size={14} />{tr("Tạo mới")}</button></div>
              <span className="hint">{tr("Máy học sinh dùng mật khẩu này để ghép với máy giáo viên")}{" "}<b>{tr("lần đầu")}</b>{tr(". Sau đó máy tự vào lớp, kể cả khi bạn đổi mật khẩu.")}
                {pwChanged && tr(" Khi lưu, mật khẩu mới sẽ được gửi tới các máy đang trực tuyến.")}</span></div>
            <Switch checked={s.accept_new} onChange={(v) => up({ accept_new: v })} label={tr("Nhận máy mới bằng mật khẩu phòng")}
                    hint={tr("Tắt khi đã đủ máy, để không ai ghép thêm máy lạ vào lớp.")} />
            <div className="field"><label>{tr("Địa chỉ máy giáo viên")}</label>
              <div className="code-box">{st.addresses.join(", ") || "—"}{" "}{tr("· cổng TCP 47900, UDP 47901")}</div>
              <span className="hint">{tr("Chỉ máy giáo viên cần mở tường lửa cho hai cổng này (bộ cài đã tự mở). Máy học sinh không cần.")}</span></div>
            <div className="field"><label>{tr("Mã giáo viên (khóa công khai)")}</label><div className="code-box">{st.teacherCode}</div></div>
          </>}
          {sec === "install" && <>
            <div className="small muted">{tr("Cài một lần trên mỗi máy học sinh, bằng tài khoản quản trị. Bộ cài gồm Agent (chạy khi học sinh đăng nhập) và dịch vụ Helper (giữ mật khẩu phòng, chặn web; học sinh không tắt được).")}</div>
            <div className="field"><label>{tr("Cách 1 – chạy file cài")}</label>
              <div className="small">{tr("Chép")}{" "}<span className="mono">labcontrol-agent.exe</span>{" "}{tr("và")}{" "}<span className="mono">cai-may-hoc-sinh.cmd</span>{" "}{tr("sang máy học sinh, chuột phải")}{" "}<b>cai-may-hoc-sinh.cmd</b>{" "}{tr("→ Run as administrator, nhập mật khẩu phòng")}{" "}<b className="mono">{s.room_password}</b>.</div></div>
            <div className="field"><label>{tr("Cách 2 – Windows (Command Prompt quản trị)")}</label><div className="code-box">{winCmd}</div>
              <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => copy(winCmd)}><Send size={13} />{tr("Chép lệnh")}</button></div>
            <div className="field"><label>Linux</label><div className="code-box">{linCmd}</div></div>
            <div className="field"><label>{tr("Khi mạng chặn tự tìm máy (khác VLAN, Wi-Fi cách ly)")}</label><div className="code-box">{ipCmd}</div>
              <span className="hint">{tr("Thêm")}{" "}<span className="mono">--teacher-ip</span>{" "}{tr("để máy học sinh nối thẳng tới IP máy giáo viên.")}</span></div>
            <div className="field"><label>{tr("Cập nhật máy học sinh từ xa")}</label>
              <div className="small muted">{tr("Không cần cài lại từng máy: máy giáo viên gửi phần mềm mới xuống, máy học sinh tự cài và khởi động lại. Bộ cài máy giáo viên đã kèm sẵn bản")}{" "}{st.version} cho {st.bundled.length ? st.bundled.map((b) => b === "windows" ? "Windows" : "Linux").join(tr(" và ")) : "—"}.</div>
              <div className="row wrap">
                <button className="btn primary" disabled={!outdated.length} onClick={() => push(null)}><RefreshCw size={14} />
                  {outdated.length ? tr("Cập nhật {0} máy dùng bản cũ", outdated.length) : tr("Mọi máy trực tuyến đã là bản mới")}</button>
                <button className="btn" disabled={!online.length} onClick={pickFile}>{tr("Dùng file Agent khác…")}</button>
              </div>
              <span className="hint">{tr("Chỉ máy đang trực tuyến được cập nhật; máy tắt sẽ hiện “bản cũ” khi vào lớp lần sau. Gói cập nhật có chữ ký của máy giáo viên, máy học sinh không nhận gói từ nơi khác.")}</span></div>
          </>}
          {sec === "lists" && <>
            <div className="field"><label>{tr("Trang web hay chặn")}</label><textarea className="input mono" rows={7} value={webText} onChange={(e) => setWebText(e.target.value)} />
              <span className="hint">{tr("Mỗi dòng một tên miền. Hiện thành nút chọn nhanh trong “Chặn web”.")}</span></div>
            <div className="field"><label>{tr("Ứng dụng hay chặn")}</label><textarea className="input mono" rows={6} value={appText} onChange={(e) => setAppText(e.target.value)} />
              <span className="hint">{tr("Tên tiến trình, vd")}{" "}<span className="mono">zalo.exe</span>{tr(". Xem tên trong Task Manager › Details.")}</span></div>
            <div className="field"><label>{tr("Ứng dụng hay mở cho học sinh")}</label><textarea className="input mono" rows={4} value={runText} onChange={(e) => setRunText(e.target.value)}
              placeholder={"C:\\Program Files (x86)\\Scratch 3\\Scratch 3.exe\nscratch-desktop"} />
              <span className="hint">{tr("Mỗi dòng một ứng dụng: tên chương trình, đường dẫn đầy đủ, hoặc lệnh có tham số. Hiện thành nút chọn nhanh trong “Mở ứng dụng”.")}</span></div>
          </>}
          {sec === "lock" && <LockSection s={s} up={up} setMedia={setMedia} />}
          {sec === "perf" && <>
            <div className="field"><label>{tr("Cập nhật ảnh màn hình mỗi")}{" "}{s.thumb_seconds}{" "}{tr("giây")}</label>
              <input type="range" min={0.5} max={10} step={0.5} value={s.thumb_seconds} onChange={(e) => up({ thumb_seconds: +e.target.value })} /></div>
            <div className="field"><label>{tr("Độ nét ảnh thu nhỏ:")}{" "}{s.thumb_width}px</label>
              <input type="range" min={200} max={800} step={40} value={s.thumb_width} onChange={(e) => up({ thumb_width: +e.target.value })} /></div>
            <div className="field"><label>{tr("Trình chiếu:")}{" "}{s.show_fps}{" "}{tr("khung hình/giây")}</label>
              <input type="range" min={2} max={30} value={s.show_fps} onChange={(e) => up({ show_fps: +e.target.value })} /></div>
            <div className="field"><label>{tr("Chất lượng trình chiếu:")}{" "}{s.show_quality}</label>
              <input type="range" min={30} max={90} step={5} value={s.show_quality} onChange={(e) => up({ show_quality: +e.target.value })} />
              <span className="hint">{tr("Mạng yếu hoặc nhiều máy: giảm khung hình/giây và chất lượng.")}</span></div>
          </>}
          {sec === "about" && <>
            <div><b>LabControl {st.version || VERSION}</b>{" "}{tr("– phần mềm quản lý phòng máy tính.")}</div>
            <div className="small muted">{tr("Đây là")}{" "}<b>{tr("phần mềm tự do")}</b>{tr(": bạn được dùng, nghiên cứu, sửa đổi và chia sẻ theo Giấy phép Công cộng GNU Affero phiên bản 3 hoặc mới hơn (AGPL-3.0-or-later). Phần mềm được cung cấp")}{" "}<b>{tr("không kèm bất kỳ bảo đảm nào")}</b>.</div>
            <div className="field"><label>{tr("Mã nguồn")}</label><div className="code-box">{SOURCE_URL}</div>
              <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => copy(SOURCE_URL)}>{tr("Chép địa chỉ")}</button></div>
            <div className="field"><label>{tr("Cập nhật phần mềm giáo viên")}</label>
              <div className="row wrap">
                <button className="btn" disabled={checking} onClick={async () => {
                  setChecking(true);
                  try {
                    const u = await api.checkUpdate();
                    toast(u.newer ? tr("Đã có bản {0}", u.version) : tr("Bạn đang dùng bản mới nhất ({0})", st.version));
                  } catch (e) { toast(errText(e), "error"); }
                  setChecking(false);
                }}><RefreshCw size={14} />{checking ? tr("Đang kiểm tra...") : tr("Kiểm tra bản mới")}</button>
                {st.update?.newer && <button className="btn primary" onClick={() => api.openRelease(st.update!.url)}><Download size={14} />{tr("Tải bản")}{" "}{st.update.version}</button>}
              </div>
              <span className="hint">{tr("LabControl tự kiểm tra khi mở và mỗi 12 giờ (cần Internet). Cài bản mới cho máy giáo viên xong, dùng “Cập nhật máy học sinh từ xa” trong mục Cài máy học sinh.")}</span></div>
            <div className="field"><label>{tr("Thư mục bài nộp và kết quả")}</label><div className="code-box">{st.outDir}</div></div>
          </>}
        </section>
      </div>
    </Modal>
  );
}

const LOOK_KEYS = ["lang", "theme_mode", "dark", "accent", "wallpaper", "glass", "glass_level", "font_scale", "radius"] as const;
function pickLook(s: Settings): Partial<Settings> {
  const o: Record<string, unknown> = {};
  for (const k of LOOK_KEYS) o[k] = s[k];
  return o as Partial<Settings>;
}
const DEFAULT_LOOK: Partial<Settings> = { theme_mode: "light", dark: false, accent: "#0a9d94", wallpaper: "aurora", glass: true, glass_level: 60, font_scale: 100, radius: "normal" };

// ------------------------------------------------------------------ màn hình khóa
const MAX_BYTES = 44 << 20;

const loadImage = (url: string) => new Promise<HTMLImageElement>((ok, bad) => {
  const im = new Image();
  im.onload = () => ok(im);
  im.onerror = () => bad(new Error(tr("Không đọc được ảnh này")));
  im.src = url;
});

/** Vẽ khung hình vào canvas (tối đa maxW × maxH, giữ tỉ lệ) rồi nén JPEG */
function toJpeg(src: CanvasImageSource, w: number, h: number, maxW: number, maxH: number, q: number): string {
  const k = Math.min(1, maxW / w, maxH / h);
  const c = document.createElement("canvas");
  c.width = Math.max(2, Math.round(w * k)); c.height = Math.max(2, Math.round(h * k));
  c.getContext("2d")!.drawImage(src, 0, 0, c.width, c.height);
  return c.toDataURL("image/jpeg", q);
}

const b64Bytes = (d: string) => Math.floor((d.length - d.indexOf(",") - 1) * 0.75);

function LockSection({ s, up, setMedia }: { s: Settings; up: (p: Partial<Settings>) => void; setMedia: (m: LockMedia) => void }) {
  const [busy, setBusy] = useState("");
  const [secs, setSecs] = useState(10);
  const [fps, setFps] = useState(10);
  const m = s.lock_media;
  const has = !!m?.sha;

  const save = async (kind: string, name: string, rate: number, frames: string[]) => {
    const thumbDataUrl = await loadImage(frames[0]).then((im) => toJpeg(im, im.naturalWidth, im.naturalHeight, 480, 270, 0.7));
    setBusy(tr("Đang lưu…"));
    const info = await api.setLockMedia(kind, name, rate, frames, thumbDataUrl);
    setMedia(info);
    toast(tr("Đã đặt ảnh nền màn hình khóa. Máy học sinh đang trực tuyến sẽ nhận trước để khóa là hiện ngay."));
  };

  const pickImage = async (f: File) => {
    const url = URL.createObjectURL(f);
    try {
      setBusy(tr("Đang xử lý ảnh…"));
      const im = await loadImage(url);
      await save("image", f.name, 0, [toJpeg(im, im.naturalWidth, im.naturalHeight, 1920, 1080, 0.85)]);
    } catch (e) { toast(errText(e), "error"); } finally { URL.revokeObjectURL(url); setBusy(""); }
  };

  const pickVideo = async (f: File) => {
    const url = URL.createObjectURL(f);
    const v = document.createElement("video");
    v.muted = true; v.preload = "auto"; v.playsInline = true;
    try {
      setBusy(tr("Đang mở video…"));
      await new Promise<void>((ok, bad) => {
        v.onloadeddata = () => ok();
        v.onerror = () => bad(new Error(tr("Máy này không mở được video đó. Hãy thử file MP4 (H.264) hoặc WebM.")));
        v.src = url;
      });
      const dur = isFinite(v.duration) && v.duration > 0 ? v.duration : secs;
      const len = Math.min(dur, secs);
      const n = Math.max(2, Math.floor(len * fps));
      const frames: string[] = [];
      let bytes = 0;
      for (let i = 0; i < n; i++) {
        await new Promise<void>((ok) => { v.onseeked = () => ok(); v.currentTime = Math.min(i / fps, dur - 0.01); });
        const fr = toJpeg(v, v.videoWidth, v.videoHeight, 1280, 720, 0.72);
        bytes += b64Bytes(fr);
        if (bytes > MAX_BYTES) throw new Error(tr("Video quá nặng: hãy giảm thời lượng hoặc số khung hình/giây."));
        frames.push(fr);
        setBusy(tr("Đang cắt khung hình… {0}%", Math.round(((i + 1) / n) * 100)));
      }
      await save("video", f.name, fps, frames);
    } catch (e) { toast(errText(e), "error"); } finally { URL.revokeObjectURL(url); v.removeAttribute("src"); setBusy(""); }
  };

  const clear = async () => {
    try { await api.clearLockMedia(); setMedia({ sha: "", kind: "", name: "", fps: 0, frames: 0, size: 0, thumb: "" }); }
    catch (e) { toast(errText(e), "error"); }
  };
  const file = (accept: string, onPick: (f: File) => void) => (
    <input type="file" accept={accept} style={{ display: "none" }}
           onChange={(e) => { const f = e.target.files?.[0]; e.target.value = ""; if (f) onPick(f); }} />
  );

  return <>
    <div className="small muted">{tr("Bấm “Khóa máy” trên thanh công cụ là khóa ngay, không cần gõ gì. Chữ và ảnh/video nền dưới đây dùng cho mọi lần khóa.")}</div>
    <div className="field"><label>{tr("Dòng chữ trên màn hình khóa")}</label>
      <input className="input" value={s.lock_message} placeholder={tr("Hãy chú ý lên giáo viên")} onChange={(e) => up({ lock_message: e.target.value })} /></div>
    <div className="field"><label>{tr("Ảnh hoặc video nền")}</label>
      <div className="lock-prev" style={has && m.thumb ? { backgroundImage: `url(${m.thumb})` } : undefined}>
        <div className="lock-band"><b>🔒 {s.lock_message.trim() || tr("Hãy chú ý lên giáo viên")}</b><span>{tr("Máy đã bị giáo viên khóa")}</span></div>
        {busy && <div className="lock-busy"><div className="spinner" />{busy}</div>}
      </div>
      {has && <div className="small faint">
        {m.kind === "video" ? <Film size={13} /> : <ImagePlus size={13} />}{" "}{m.name}{" · "}
        {m.kind === "video" ? tr("{0} khung · {1} khung/giây · ", m.frames, m.fps) : ""}{m.size >= 1048576 ? `${(m.size / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(m.size / 1024))} KB`}</div>}
      <div className="row wrap">
        <label className={`btn ${busy ? "disabled" : ""}`}><ImagePlus size={14} />{tr("Chọn ảnh…")}{!busy && file("image/*", pickImage)}</label>
        <label className={`btn ${busy ? "disabled" : ""}`}><Film size={14} />{tr("Chọn video…")}{!busy && file("video/*", pickVideo)}</label>
        {has && <button className="btn" disabled={!!busy} onClick={clear}><Trash2 size={14} />{tr("Bỏ ảnh nền")}</button>}
      </div>
      <span className="hint">{tr("Ảnh: JPG, PNG, WebP… Video được cắt thành chuỗi khung hình, chạy lặp lại, không có tiếng. Nên dùng đoạn ngắn để gửi nhanh qua mạng.")}</span></div>
    <div className="field"><label>{tr("Video: lấy tối đa {0} giây đầu, {1} khung hình/giây", secs, fps)}</label>
      <div className="row wrap">
        <input type="range" min={3} max={30} value={secs} onChange={(e) => setSecs(+e.target.value)} style={{ flex: 1 }} />
        <select className="sel" value={fps} onChange={(e) => setFps(+e.target.value)}>
          {[6, 8, 10, 12, 15].map((x) => <option key={x} value={x}>{tr("{0} khung/giây", x)}</option>)}
        </select>
      </div>
      <span className="hint">{tr("Áp dụng cho lần chọn video tiếp theo. Máy học sinh cần bản 3.3.0 trở lên để hiện ảnh/video nền.")}</span></div>
  </>;
}

function LookSection({ s, look }: { s: Settings; look: (p: Partial<Settings>) => void }) {
  const en = lang() === "en";
  const mode = s.theme_mode || (s.dark ? "dark" : "light");
  const lv = s.glass === false ? 0 : (s.glass_level ?? 60);
  const accent = (s.accent || "#0a9d94").toLowerCase();
  const seg = <T extends string | number>(cur: T, items: [T, string][], f: (v: T) => void) => (
    <div className="seg" style={{ alignSelf: "flex-start" }}>{items.map(([v, l]) =>
      <button key={String(v)} className={cur === v ? "active" : ""} style={{ padding: "0 14px" }} onClick={() => f(v)}>{l}</button>)}</div>);
  return <>
    <div className="field"><label>{tr("Ngôn ngữ")}</label>
      <select className="input" style={{ maxWidth: 260 }} value={s.lang || "vi"} onChange={(e) => look({ lang: e.target.value })}>
        {LANGS.map((l) => <option key={l.code} value={l.code}>{l.name}</option>)}
      </select>
      <span className="hint">{tr("Áp dụng cho cả máy học sinh: màn hình khóa, tin nhắn, bài trắc nghiệm, thông báo.")}</span></div>
    <div className="field"><label>{tr("Chế độ")}</label>
      {seg(mode, [["light", tr("Sáng")], ["dark", tr("Tối")], ["system", tr("Theo hệ thống")]], (v) => look({ theme_mode: v, dark: v === "dark" }))}</div>
    <div className="field"><label>{tr("Màu nhấn")}</label>
      <div className="swatches">
        {ACCENTS.map((a) => <button key={a.hex} className={`swatch ${accent === a.hex ? "on" : ""}`} style={{ background: a.hex }}
                                    title={en ? a.en : a.name} onClick={() => look({ accent: a.hex })} />)}
        <label className={`swatch custom ${ACCENTS.some((a) => a.hex === accent) ? "" : "on"}`} title={tr("Tự chọn màu")}>
          <input type="color" value={accent} onChange={(e) => look({ accent: e.target.value })} /></label>
      </div></div>
    <div className="field"><label>{tr("Hình nền")}</label>
      <div className="swatches">
        {Object.entries(WALLPAPERS).map(([k, w]) => {
          const c = mode === "dark" ? w.dark : w.light;
          return <button key={k} className={`wall ${(s.wallpaper || "aurora") === k ? "on" : ""}`} onClick={() => look({ wallpaper: k })}
                         style={{ background: `radial-gradient(circle at 15% 10%, ${c[1]}, transparent 70%), radial-gradient(circle at 90% 0%, ${c[2]}, transparent 60%), ${c[0]}` }}>
            {en ? w.en : w.name}</button>;
        })}
      </div></div>
    <div className="field"><label>{tr("Hiệu ứng kính trong suốt")}: {lv === 0 ? tr("tắt") : `${lv}%`}</label>
      <input type="range" min={0} max={100} step={10} value={lv} style={{ maxWidth: 360 }}
             onChange={(e) => { const v = +e.target.value; look({ glass_level: v, glass: v > 0 }); }} />
      <span className="hint">{tr("Kéo về 0 nếu máy giáo viên yếu hoặc giao diện bị giật.")}</span></div>
    <div className="field"><label>{tr("Cỡ giao diện")}</label>
      {seg(s.font_scale || 100, [[90, "90%"], [100, "100%"], [110, "110%"], [125, "125%"]], (v) => look({ font_scale: v }))}</div>
    <div className="field"><label>{tr("Bo góc")}</label>
      {seg(s.radius || "normal", [["compact", tr("Ít")], ["normal", tr("Vừa")], ["round", tr("Nhiều")]], (v) => look({ radius: v }))}</div>
    <button className="btn" style={{ alignSelf: "flex-start" }} onClick={() => look(DEFAULT_LOOK)}><RotateCcw size={14} />{tr("Khôi phục mặc định")}</button>
  </>;
}
