//! Tắt / bật tiếng loa của máy học sinh (thiết bị phát mặc định).
//!  - Windows: Core Audio (IAudioEndpointVolume)
//!  - Linux: pactl (PulseAudio / PipeWire), dự phòng amixer (ALSA)

#[cfg(windows)]
mod imp {
    use anyhow::Result;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    fn endpoint() -> windows::core::Result<IAudioEndpointVolume> {
        unsafe {
            // gọi nhiều lần trên cùng luồng vẫn an toàn (lần sau trả về S_FALSE)
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole)?;
            dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
        }
    }

    pub fn set_mute(on: bool) -> Result<()> {
        unsafe { endpoint()?.SetMute(on, std::ptr::null())? };
        Ok(())
    }

    pub fn is_muted() -> Option<bool> {
        unsafe { endpoint().ok()?.GetMute().ok().map(|b| b.as_bool()) }
    }
}

#[cfg(not(windows))]
mod imp {
    use anyhow::{bail, Result};
    use std::process::{Command, Stdio};

    fn run(cmd: &str, args: &[&str]) -> Option<String> {
        let o = Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
    }

    pub fn set_mute(on: bool) -> Result<()> {
        if run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", if on { "1" } else { "0" }]).is_some() {
            return Ok(());
        }
        if run("amixer", &["-q", "sset", "Master", if on { "mute" } else { "unmute" }]).is_some() {
            return Ok(());
        }
        bail!("không điều khiển được âm thanh (thiếu pactl / amixer)")
    }

    pub fn is_muted() -> Option<bool> {
        if let Some(s) = run("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]) {
            return Some(s.to_lowercase().contains("yes"));
        }
        run("amixer", &["get", "Master"]).map(|s| s.contains("[off]"))
    }
}

pub use imp::{is_muted, set_mute};
