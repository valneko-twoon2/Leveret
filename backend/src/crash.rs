//! Last-resort error handling: panics are written to a log file, and fatal
//! startup errors are shown in a message box instead of the app silently vanishing.

use std::io::Write;
use std::path::PathBuf;

/// `%LOCALAPPDATA%\com.leveret.desktop\logs`
pub fn logs_dir() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(base).join("com.leveret.desktop").join("logs"))
}

fn crash_log() -> Option<PathBuf> {
    Some(logs_dir()?.join("crash.log"))
}

/// Appends every panic (message, location, backtrace) to the crash log, then
/// runs the default hook. Panics inside commands are turned into errors by the
/// async runtime, so the app keeps running.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = crash_log() {
            let _ = (|| -> std::io::Result<()> {
                std::fs::create_dir_all(path.parent().unwrap_or(&path))?;
                let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let backtrace = std::backtrace::Backtrace::force_capture();
                writeln!(f, "[{stamp}] {} {info}\n{backtrace}\n", env!("CARGO_PKG_VERSION"))
            })();
        }
        default(info);
    }));
}

/// Shows `message` in a message box and exits.
pub fn fatal(message: &str) -> ! {
    log::error!("{message}");
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        use windows::core::PCWSTR;
        let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
        let (text, caption) = (wide(message), wide("Leveret"));
        // SAFETY: both strings are NUL-terminated and outlive the call.
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(caption.as_ptr()),
                MB_OK | MB_ICONERROR,
            )
        };
    }
    #[cfg(not(windows))]
    eprintln!("{message}");
    std::process::exit(1)
}
