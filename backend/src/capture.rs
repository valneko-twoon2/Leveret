//! Screenshot support.
//!
//! `leveret --capture <dir> [--mock] [--theme light|dark] [--scenes a,b]` starts the app off-screen
//! (no taskbar entry, no focus), lets the frontend walk through its capture
//! scenes and saves each one as `<dir>/<scene>.png`, then exits.
//! The current screen can also be saved at any time with Ctrl+Shift+S.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub dir: String,
    /// Scene ids to capture; all scenes when `None`.
    pub scenes: Option<Vec<String>>,
}

/// Command line options passed to the frontend.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupOptions {
    /// `--mock`: use the frontend's built-in mock data instead of real repositories.
    pub mock: bool,
    /// `--theme light|dark`: force a color theme instead of following the OS.
    pub theme: Option<String>,
    pub capture: Option<CaptureRequest>,
}

/// Parses `--capture <dir>`, `--mock`, `--theme <light|dark>` and `--scenes a,b` from the command line.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> StartupOptions {
    let mut dir = None;
    let mut mock = false;
    let mut theme = None;
    let mut scenes = None;
    let mut args = args.into_iter().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--capture" => dir = args.next(),
            "--mock" => mock = true,
            "--theme" => theme = args.next().filter(|t| t == "light" || t == "dark"),
            "--scenes" => {
                scenes = args.next().map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
            }
            _ => {}
        }
    }
    let capture = dir.map(|dir| {
        let dir = std::path::absolute(&dir).unwrap_or_else(|_| PathBuf::from(&dir));
        CaptureRequest { dir: dir.display().to_string(), scenes }
    });
    StartupOptions { mock, theme, capture }
}

/// Saves the webview's current rendering to `path` as PNG.
#[cfg(windows)]
pub async fn capture_webview(window: &tauri::WebviewWindow, path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use webview2_com::CapturePreviewCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
    use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
    use windows::Win32::System::Com::{STGC_DEFAULT, STGM_CREATE, STGM_WRITE};
    use windows::Win32::UI::Shell::SHCreateStreamOnFileEx;
    use windows::core::PCWSTR;

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let (tx, rx) = std::sync::mpsc::channel::<std::result::Result<(), String>>();

    window
        .with_webview(move |webview| {
            let start = || -> windows::core::Result<()> {
                unsafe {
                    let core = webview.controller().CoreWebView2()?;
                    let stream = SHCreateStreamOnFileEx(
                        PCWSTR(wide.as_ptr()),
                        (STGM_CREATE | STGM_WRITE).0,
                        FILE_ATTRIBUTE_NORMAL.0,
                        true,
                        None,
                    )?;
                    let done_stream = stream.clone();
                    let done_tx = tx.clone();
                    let handler = CapturePreviewCompletedHandler::create(Box::new(move |result| {
                        let result = result.and_then(|_| done_stream.Commit(STGC_DEFAULT));
                        // Releasing the last reference closes the file.
                        drop(done_stream);
                        let _ = done_tx.send(result.map_err(|e| e.to_string()));
                        Ok(())
                    }));
                    core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, &stream, &handler)
                }
            };
            if let Err(e) = start() {
                let _ = tx.send(Err(e.to_string()));
            }
        })
        .map_err(|e| AppError::Other(e.to_string()))?;

    tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
        .map_err(|e| AppError::Other(e.to_string()))?
        .map_err(AppError::Other)
}

#[cfg(not(windows))]
pub async fn capture_webview(_window: &tauri::WebviewWindow, _path: &Path) -> Result<()> {
    Err(AppError::Other("screen capture is only supported on Windows".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_capture_args() {
        let args = ["leveret", "--capture", "out", "--mock", "--theme", "dark", "--scenes", "a, b"].map(String::from);
        let opts = parse_args(args);
        assert!(opts.mock);
        assert_eq!(opts.theme.as_deref(), Some("dark"));
        let req = opts.capture.unwrap();
        assert!(req.dir.ends_with("out"));
        assert_eq!(req.scenes, Some(vec!["a".to_string(), "b".to_string()]));
        let none = parse_args(["leveret".to_string()]);
        assert!(none.capture.is_none() && !none.mock);
    }
}
