//! Commands for the app itself (startup options, screenshots, exiting) and the
//! creation of the main window.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::capture::{self, StartupOptions};
use crate::error::{AppError, Result};
use crate::{lockdown, security};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FrontendOptions {
    #[serde(flatten)]
    startup: StartupOptions,
    devtools: bool,
}

#[tauri::command]
pub fn startup_options(options: State<'_, StartupOptions>) -> FrontendOptions {
    FrontendOptions {
        startup: options.inner().clone(),
        devtools: lockdown::devtools_allowed(),
    }
}

/// Saves the current screen. `path` is only accepted in capture mode and must be a
/// PNG inside the capture folder; otherwise the file goes to `Pictures\Leveret`.
/// Returns the saved path.
#[tauri::command]
pub async fn capture_screen(
    app: AppHandle,
    window: WebviewWindow,
    options: State<'_, StartupOptions>,
    path: Option<String>,
) -> Result<String> {
    let path = match path {
        Some(p) => {
            let p = PathBuf::from(p);
            let dir = options.capture.as_ref().map(|c| PathBuf::from(&c.dir));
            let file_ok = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".png") && security::folder_name(n).is_ok());
            if dir.is_none() || p.parent() != dir.as_deref() || !file_ok {
                return Err(AppError::InvalidInput(p.display().to_string()));
            }
            p
        }
        None => {
            let dir = app.path().picture_dir().map_err(|e| AppError::Other(e.to_string()))?;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis());
            dir.join("Leveret").join(format!("leveret-{stamp}.png"))
        }
    };
    capture::capture_webview(&window, &path).await?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn exit_app(app: AppHandle, code: i32) {
    app.exit(code);
}

pub(crate) fn create_main_window(app: &tauri::App, capture: bool) -> tauri::Result<()> {
    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("Leveret")
        .inner_size(1280.0, 800.0)
        .min_inner_size(900.0, 560.0);
    builder = lockdown::harden(builder);
    if capture {
        // Render in the background: off-screen, no taskbar entry, no focus.
        // Chromium stops painting windows it considers occluded, so turn that off,
        // and use a separate profile so a running Leveret (with different browser
        // arguments) doesn't conflict.
        let data_dir = app.path().app_local_data_dir()?.join("capture-webview");
        builder = builder
            .position(-32000.0, -32000.0)
            .skip_taskbar(true)
            .focused(false)
            .data_directory(data_dir)
            .additional_browser_args(
                "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion \
                 --disable-backgrounding-occluded-windows --disable-renderer-backgrounding",
            );
    } else {
        // Shown once the page has loaded, to avoid a white flash.
        builder = builder.visible(false);
    }
    let window = builder.build()?;
    lockdown::apply_webview_settings(&window)?;
    Ok(())
}
