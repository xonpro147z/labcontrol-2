import { useState, type ReactNode } from "react";
import { Ban, Download, Gauge, Info, KeyRound, Laptop, RefreshCw, Send } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, reload, setSettingsLocal, useStore, type Settings } from "../api";
import { SOURCE_URL, VERSION } from "../components/About";
import { Modal, Switch, errText, toast } from "../components/ui";

const SECTIONS: { k: string; label: string; icon: ReactNode }[] = [
  { k: "room", label: "Phòng & ghép máy", icon: <KeyRound size={16} /> },
  { k: "install", label: "Cài máy học sinh", icon: <Laptop size={16} /> },
  { k: "lists", label: "Danh sách chặn", icon: <Ban size={16} /> },
  { k: "perf", label: "Hình ảnh & tốc độ", icon: <Gauge size={16} /> },
  { k: "about", label: "Giới thiệu", icon: <Info size={16} /> },
];

function randomPw() {
  const a = new Uint32Array(1);
  crypto.getRandomValues(a);
  return String(100000 + (a[0] % 900000));
}

export function SettingsModal({ initial, onClose }: { initial: string; onClose: () => void }) {
  const st = useStore();
  const [sec, setSec] = useState(initial);
  const [s, setS] = useState<Settings>(st.settings!);
  const [webText, setWebText] = useState(st.settings!.web_list.join("\n"));
  const [appText, setAppText] = useState(st.settings!.app_list.join("\n"));
  const [runText, setRunText] = useState((st.settings!.run_list ?? []).join("\n"));
  const [err, setErr] = useState("");
  const pwChanged = s.room_password.trim() !== st.settings!.room_password;
  const up = (p: Partial<Settings>) => setS({ ...s, ...p });

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
        toast(`Đã lưu. Mật khẩu phòng mới đã gửi tới ${n} máy đang trực tuyến`);
      } else toast("Đã lưu cài đặt");
      await reload();
      onClose();
    } catch (e) { setErr(errText(e)); }
  };

  const room = s.room.trim() ? ` --room "${s.room.trim()}"` : "";
  const winCmd = `labcontrol-agent.exe install --password "${s.room_password.trim()}"${room}`;
  const linCmd = `sudo ./labcontrol-agent install --password "${s.room_password.trim()}"${room}`;
  const ipCmd = `${winCmd} --teacher-ip ${st.addresses[0] ?? "<IP máy giáo viên>"}`;
  const [checking, setChecking] = useState(false);
  const online = [...st.students.values()].filter((x) => x.state === "online");
  const outdated = online.filter((x) => x.info.version && x.info.version !== st.version);
  const push = async (file: string | null) => {
    try {
      const n = await api.pushUpdate((file ? online : outdated).map((x) => x.id), file);
      toast(n ? `Đang gửi bản cập nhật tới ${n} máy` : "Không có máy nào cần cập nhật");
    } catch (e) { toast(errText(e), "error"); }
  };
  const pickFile = async () => {
    const f = await open({ multiple: false, title: "Chọn file labcontrol-agent (.exe cho Windows, không đuôi cho Linux)" });
    if (f) push(String(f));
  };
  const copy = (t: string) => navigator.clipboard.writeText(t).then(() => toast("Đã chép"));

  return (
    <Modal title="Cài đặt" onClose={onClose} wide bare footer={<>
      {err && <span className="err" style={{ marginRight: "auto" }}>{err}</span>}
      <button className="btn" onClick={onClose}>Hủy</button><button className="btn primary" onClick={save}>Lưu</button></>}>
      <div className="settings">
        <nav>{SECTIONS.map((x) => <button key={x.k} className={sec === x.k ? "active" : ""} onClick={() => setSec(x.k)}>{x.icon}{x.label}</button>)}</nav>
        <section>
          {sec === "room" && <>
            <div className="field"><label>Tên giáo viên</label><input className="input" value={s.teacher_name} onChange={(e) => up({ teacher_name: e.target.value })} /></div>
            <div className="field"><label>Tên phòng máy</label><input className="input" placeholder="vd: P.Tin 1 (để trống nếu mạng chỉ có một phòng)" value={s.room} onChange={(e) => up({ room: e.target.value })} />
              <span className="hint">Khi nhiều phòng dùng chung mạng, đặt tên phòng để máy học sinh chỉ vào đúng phòng của mình. Tên phòng phải giống lúc cài máy học sinh.</span></div>
            <div className="field"><label>Mật khẩu phòng</label>
              <div className="row"><input className="input mono" style={{ fontSize: 16, letterSpacing: ".08em" }} value={s.room_password} onChange={(e) => up({ room_password: e.target.value })} />
                <button className="btn" onClick={() => up({ room_password: randomPw() })}><RefreshCw size={14} />Tạo mới</button></div>
              <span className="hint">Máy học sinh dùng mật khẩu này để ghép với máy giáo viên <b>lần đầu</b>. Sau đó máy tự vào lớp, kể cả khi bạn đổi mật khẩu.
                {pwChanged && " Khi lưu, mật khẩu mới sẽ được gửi tới các máy đang trực tuyến."}</span></div>
            <Switch checked={s.accept_new} onChange={(v) => up({ accept_new: v })} label="Nhận máy mới bằng mật khẩu phòng"
                    hint="Tắt khi đã đủ máy, để không ai ghép thêm máy lạ vào lớp." />
            <div className="field"><label>Địa chỉ máy giáo viên</label>
              <div className="code-box">{st.addresses.join(", ") || "—"} · cổng TCP 47900, UDP 47901</div>
              <span className="hint">Chỉ máy giáo viên cần mở tường lửa cho hai cổng này (bộ cài đã tự mở). Máy học sinh không cần.</span></div>
            <div className="field"><label>Mã giáo viên (khóa công khai)</label><div className="code-box">{st.teacherCode}</div></div>
          </>}
          {sec === "install" && <>
            <div className="small muted">Cài một lần trên mỗi máy học sinh, bằng tài khoản quản trị. Bộ cài gồm Agent (chạy khi học sinh đăng nhập)
              và dịch vụ Helper (giữ mật khẩu phòng, chặn web; học sinh không tắt được).</div>
            <div className="field"><label>Cách 1 – chạy file cài</label>
              <div className="small">Chép <span className="mono">labcontrol-agent.exe</span> và <span className="mono">cai-may-hoc-sinh.cmd</span> sang máy học sinh,
                chuột phải <b>cai-may-hoc-sinh.cmd</b> → Run as administrator, nhập mật khẩu phòng <b className="mono">{s.room_password}</b>.</div></div>
            <div className="field"><label>Cách 2 – Windows (Command Prompt quản trị)</label><div className="code-box">{winCmd}</div>
              <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => copy(winCmd)}><Send size={13} />Chép lệnh</button></div>
            <div className="field"><label>Linux</label><div className="code-box">{linCmd}</div></div>
            <div className="field"><label>Khi mạng chặn tự tìm máy (khác VLAN, Wi-Fi cách ly)</label><div className="code-box">{ipCmd}</div>
              <span className="hint">Thêm <span className="mono">--teacher-ip</span> để máy học sinh nối thẳng tới IP máy giáo viên.</span></div>
            <div className="field"><label>Cập nhật máy học sinh từ xa</label>
              <div className="small muted">Không cần cài lại từng máy: máy giáo viên gửi phần mềm mới xuống, máy học sinh tự cài và khởi động lại.
                Bộ cài máy giáo viên đã kèm sẵn bản {st.version} cho {st.bundled.length ? st.bundled.map((b) => b === "windows" ? "Windows" : "Linux").join(" và ") : "—"}.</div>
              <div className="row wrap">
                <button className="btn primary" disabled={!outdated.length} onClick={() => push(null)}><RefreshCw size={14} />
                  {outdated.length ? `Cập nhật ${outdated.length} máy dùng bản cũ` : "Mọi máy trực tuyến đã là bản mới"}</button>
                <button className="btn" disabled={!online.length} onClick={pickFile}>Dùng file Agent khác…</button>
              </div>
              <span className="hint">Chỉ máy đang trực tuyến được cập nhật; máy tắt sẽ hiện “bản cũ” khi vào lớp lần sau. Gói cập nhật có chữ ký của máy giáo viên, máy học sinh không nhận gói từ nơi khác.</span></div>
          </>}
          {sec === "lists" && <>
            <div className="field"><label>Trang web hay chặn</label><textarea className="input mono" rows={7} value={webText} onChange={(e) => setWebText(e.target.value)} />
              <span className="hint">Mỗi dòng một tên miền. Hiện thành nút chọn nhanh trong “Chặn web”.</span></div>
            <div className="field"><label>Ứng dụng hay chặn</label><textarea className="input mono" rows={6} value={appText} onChange={(e) => setAppText(e.target.value)} />
              <span className="hint">Tên tiến trình, vd <span className="mono">zalo.exe</span>. Xem tên trong Task Manager › Details.</span></div>
            <div className="field"><label>Ứng dụng hay mở cho học sinh</label><textarea className="input mono" rows={4} value={runText} onChange={(e) => setRunText(e.target.value)}
              placeholder={"C:\\Program Files (x86)\\Scratch 3\\Scratch 3.exe\nscratch-desktop"} />
              <span className="hint">Mỗi dòng một ứng dụng: tên chương trình, đường dẫn đầy đủ, hoặc lệnh có tham số. Hiện thành nút chọn nhanh trong “Mở ứng dụng”.</span></div>
          </>}
          {sec === "perf" && <>
            <div className="field"><label>Cập nhật ảnh màn hình mỗi {s.thumb_seconds} giây</label>
              <input type="range" min={0.5} max={10} step={0.5} value={s.thumb_seconds} onChange={(e) => up({ thumb_seconds: +e.target.value })} /></div>
            <div className="field"><label>Độ nét ảnh thu nhỏ: {s.thumb_width}px</label>
              <input type="range" min={200} max={800} step={40} value={s.thumb_width} onChange={(e) => up({ thumb_width: +e.target.value })} /></div>
            <div className="field"><label>Trình chiếu: {s.show_fps} khung hình/giây</label>
              <input type="range" min={2} max={30} value={s.show_fps} onChange={(e) => up({ show_fps: +e.target.value })} /></div>
            <div className="field"><label>Chất lượng trình chiếu: {s.show_quality}</label>
              <input type="range" min={30} max={90} step={5} value={s.show_quality} onChange={(e) => up({ show_quality: +e.target.value })} />
              <span className="hint">Mạng yếu hoặc nhiều máy: giảm khung hình/giây và chất lượng.</span></div>
            <Switch checked={s.glass !== false} onChange={(v) => up({ glass: v })} label="Hiệu ứng kính trong suốt"
                    hint="Tắt nếu máy giáo viên yếu hoặc giao diện bị giật." />
          </>}
          {sec === "about" && <>
            <div><b>LabControl {st.version || VERSION}</b> – phần mềm quản lý phòng máy tính.</div>
            <div className="small muted">Đây là <b>phần mềm tự do</b>: bạn được dùng, nghiên cứu, sửa đổi và chia sẻ theo Giấy phép Công cộng GNU Affero
              phiên bản 3 hoặc mới hơn (AGPL-3.0-or-later). Phần mềm được cung cấp <b>không kèm bất kỳ bảo đảm nào</b>.</div>
            <div className="field"><label>Mã nguồn</label><div className="code-box">{SOURCE_URL}</div>
              <button className="btn sm" style={{ alignSelf: "flex-start" }} onClick={() => copy(SOURCE_URL)}>Chép địa chỉ</button></div>
            <div className="field"><label>Cập nhật phần mềm giáo viên</label>
              <div className="row wrap">
                <button className="btn" disabled={checking} onClick={async () => {
                  setChecking(true);
                  try {
                    const u = await api.checkUpdate();
                    toast(u.newer ? `Đã có bản ${u.version}` : `Bạn đang dùng bản mới nhất (${st.version})`);
                  } catch (e) { toast(errText(e), "error"); }
                  setChecking(false);
                }}><RefreshCw size={14} />{checking ? "Đang kiểm tra..." : "Kiểm tra bản mới"}</button>
                {st.update?.newer && <button className="btn primary" onClick={() => api.openRelease(st.update!.url)}><Download size={14} />Tải bản {st.update.version}</button>}
              </div>
              <span className="hint">LabControl tự kiểm tra khi mở và mỗi 12 giờ (cần Internet). Cài bản mới cho máy giáo viên xong, dùng “Cập nhật máy học sinh từ xa” trong mục Cài máy học sinh.</span></div>
            <div className="field"><label>Thư mục bài nộp và kết quả</label><div className="code-box">{st.outDir}</div></div>
          </>}
        </section>
      </div>
    </Modal>
  );
}
