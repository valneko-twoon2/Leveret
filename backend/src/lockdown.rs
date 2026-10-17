//! Turns the WebView into an application surface: no browser shortcuts,
//! devtools, zoom, context menu, navigation away from the app, new windows or
//! downloads. The frontend adds a keyboard/mouse guard on top
//! (frontend/src/lib/lockdown.ts) as a second layer.

use tauri::webview::NewWindowResponse;
use tauri::{Runtime, Url, WebviewWindow, WebviewWindowBuilder};

/// Devtools are off in every build. Debug builds can opt in with `LEVERET_DEVTOOLS=1`.
pub fn devtools_allowed() -> bool {
    cfg!(debug_assertions) && std::env::var("LEVERET_DEVTOOLS").is_ok_and(|v| v == "1")
}

/// Whether `url` is the app itself (bundled assets, or the Vite dev server in debug builds).
pub fn is_app_url(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => {
            url.host_str() == Some("tauri.localhost")
                || (cfg!(debug_assertions) && url.host_str() == Some("localhost") && url.port() == Some(1420))
        }
        _ => false,
    }
}

pub fn harden<'a, R: Runtime, M: tauri::Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    builder
        .devtools(devtools_allowed())
        .zoom_hotkeys_enabled(false)
        .browser_extensions_enabled(false)
        .general_autofill_enabled(false)
        .on_navigation(|url| {
            let allowed = is_app_url(url);
            if !allowed {
                log::warn!("blocked navigation to {url}");
            }
            allowed
        })
        .on_new_window(|url, _| {
            log::warn!("blocked new window for {url}");
            NewWindowResponse::Deny
        })
        .on_download(|_, _| false)
}

/// WebView2 settings Tauri does not expose: browser accelerator keys (F5, F12,
/// Ctrl+P, Ctrl+F, Ctrl+U, ...), the default context menu and password autosave.
#[cfg(windows)]
pub fn apply_webview_settings<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Settings3, ICoreWebView2Settings4};
    use windows::core::Interface;

    let devtools = devtools_allowed();
    window.with_webview(move |webview| {
        let apply = || -> windows::core::Result<()> {
            unsafe {
                let settings = webview.controller().CoreWebView2()?.Settings()?;
                settings.SetAreDefaultContextMenusEnabled(false)?;
                settings.SetAreHostObjectsAllowed(false)?;
                settings.SetIsStatusBarEnabled(false)?;
                if let Ok(s3) = settings.cast::<ICoreWebView2Settings3>() {
                    // Keep F12 / Ctrl+Shift+I when devtools were explicitly enabled.
                    s3.SetAreBrowserAcceleratorKeysEnabled(devtools)?;
                }
                if let Ok(s4) = settings.cast::<ICoreWebView2Settings4>() {
                    s4.SetIsPasswordAutosaveEnabled(false)?;
                    s4.SetIsGeneralAutofillEnabled(false)?;
                }
            }
            Ok(())
        };
        if let Err(e) = apply() {
            log::error!("failed to apply WebView2 settings: {e}");
        }
    })
}

#[cfg(not(windows))]
pub fn apply_webview_settings<R: Runtime>(_window: &WebviewWindow<R>) -> tauri::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_urls() {
        assert!(is_app_url(&Url::parse("http://tauri.localhost/index.html").unwrap()));
        assert!(is_app_url(&Url::parse("tauri://localhost/").unwrap()));
        assert!(!is_app_url(&Url::parse("https://github.com/").unwrap()));
        assert!(!is_app_url(&Url::parse("file:///C:/Windows/win.ini").unwrap()));
        assert!(!is_app_url(&Url::parse("http://localhost:8080/").unwrap()));
        assert!(!is_app_url(&Url::parse("http://tauri.localhost.evil.com/").unwrap()));
    }
}
