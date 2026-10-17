mod capture;
mod error;
mod git;
mod github;
mod lockdown;
mod repos;
mod security;

use std::path::PathBuf;

use serde::Serialize;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use capture::StartupOptions;
use error::{AppError, Result};
use git::diff::FileDiff;
use git::log::{CommitDetail, LogEntry, LogScope};
use git::status::RepoStatus;
use repos::{RepoEntry, RepoList, RepoStore};

/// Upper bound for the number of log entries requested at once.
const MAX_LOG_LIMIT: u32 = 100_000;

/// Runs blocking work (git, network) off the async runtime's worker threads.
async fn blocking<T, F>(f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
}

// --- repositories -----------------------------------------------------------

#[tauri::command]
fn list_repos(store: State<'_, RepoStore>) -> RepoList {
    store.list()
}

#[tauri::command]
async fn add_repo(app: AppHandle, path: String) -> Result<RepoEntry> {
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err(AppError::InvalidInput(path.display().to_string()));
    }
    blocking(move || app.state::<RepoStore>().add(&path)).await
}

#[tauri::command]
fn remove_repo(store: State<'_, RepoStore>, path: String) -> Result<()> {
    store.remove(&path)
}

#[tauri::command]
fn select_repo(store: State<'_, RepoStore>, path: Option<String>) -> Result<()> {
    store.select(path)
}

// --- changes & history ------------------------------------------------------

#[tauri::command]
async fn get_status(store: State<'_, RepoStore>, repo: String) -> Result<RepoStatus> {
    let repo = store.require(&repo)?;
    blocking(move || git::status::get(&repo)).await
}

#[tauri::command]
async fn get_working_diff(
    store: State<'_, RepoStore>,
    repo: String,
    path: String,
    orig_path: Option<String>,
    untracked: bool,
) -> Result<FileDiff> {
    let repo = store.require(&repo)?;
    security::relative_path(&path)?;
    if let Some(o) = &orig_path {
        security::relative_path(o)?;
    }
    blocking(move || git::diff::working(&repo, &path, orig_path.as_deref(), untracked)).await
}

#[tauri::command]
async fn commit(store: State<'_, RepoStore>, repo: String, request: git::commit::CommitRequest) -> Result<String> {
    let repo = store.require(&repo)?;
    for p in &request.paths {
        security::relative_path(p)?;
    }
    blocking(move || git::commit::commit(&repo, &request)).await
}

#[tauri::command]
async fn get_log(store: State<'_, RepoStore>, repo: String, scope: LogScope, limit: u32) -> Result<Vec<LogEntry>> {
    let repo = store.require(&repo)?;
    let limit = limit.clamp(1, MAX_LOG_LIMIT);
    blocking(move || git::log::get(&repo, scope, limit)).await
}

#[tauri::command]
async fn get_commit(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<CommitDetail> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || git::log::detail(&repo, &hash)).await
}

#[tauri::command]
async fn get_commit_diff(
    store: State<'_, RepoStore>,
    repo: String,
    hash: String,
    parent: Option<String>,
    path: String,
    orig_path: Option<String>,
) -> Result<FileDiff> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    if let Some(p) = &parent {
        security::object_id(p)?;
    }
    security::relative_path(&path)?;
    if let Some(o) = &orig_path {
        security::relative_path(o)?;
    }
    blocking(move || git::diff::commit_file(&repo, &hash, parent.as_deref(), &path, orig_path.as_deref())).await
}

// --- GitHub -----------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GithubState {
    account: Option<github::Account>,
    /// Whether the OAuth device flow is available (an OAuth App client id is configured).
    device_flow: bool,
}

#[tauri::command]
async fn github_state() -> Result<GithubState> {
    let account = blocking(github::account).await?;
    Ok(GithubState { account, device_flow: github::client_id().is_some() })
}

#[tauri::command]
async fn github_start_sign_in() -> Result<github::DeviceCode> {
    blocking(github::start_device_flow).await
}

#[tauri::command]
async fn github_poll_sign_in(device_code: String) -> Result<github::PollResult> {
    if device_code.is_empty() || device_code.len() > 256 || !device_code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(AppError::InvalidInput("device code".into()));
    }
    blocking(move || github::poll_device_flow(&device_code)).await
}

#[tauri::command]
async fn github_sign_in_with_token(token: String) -> Result<github::Account> {
    blocking(move || github::sign_in_with_token(&token)).await
}

#[tauri::command]
async fn github_sign_out() -> Result<()> {
    blocking(github::sign_out).await
}

#[tauri::command]
async fn github_list_repos() -> Result<Vec<github::GithubRepo>> {
    blocking(github::list_repos).await
}

/// Opens a github.com page (sign-in page, repository page) in the default browser.
#[tauri::command]
fn open_github_url(app: AppHandle, url: String) -> Result<()> {
    security::github_web_url(&url)?;
    app.opener().open_url(url, None::<&str>).map_err(|e| AppError::Other(e.to_string()))
}

// --- clone ------------------------------------------------------------------

#[tauri::command]
fn default_clone_dir(app: AppHandle) -> Result<String> {
    let docs = app.path().document_dir().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(docs.join("GitHub").display().to_string())
}

/// Clones `url` into `<parent>\<name>` and registers it. Progress is emitted as
/// `clone-progress` events.
#[tauri::command]
async fn clone_repo(app: AppHandle, window: WebviewWindow, url: String, parent: String, name: String) -> Result<RepoEntry> {
    security::clone_url(&url)?;
    security::folder_name(&name)?;
    let parent = PathBuf::from(parent);
    if !parent.is_absolute() || parent.as_os_str().to_string_lossy().chars().any(char::is_control) {
        return Err(AppError::InvalidInput(parent.display().to_string()));
    }
    let dest = parent.join(&name);
    blocking(move || {
        let envs = github::git_auth_env(&url);
        git::clone::clone(&url, &dest, envs, |p| {
            let _ = window.emit("clone-progress", p);
        })?;
        app.state::<RepoStore>().add(&dest)
    })
    .await
}

// --- app --------------------------------------------------------------------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FrontendOptions {
    #[serde(flatten)]
    startup: StartupOptions,
    devtools: bool,
}

#[tauri::command]
fn startup_options(options: State<'_, StartupOptions>) -> FrontendOptions {
    FrontendOptions { startup: options.inner().clone(), devtools: lockdown::devtools_allowed() }
}

/// Saves the current screen. `path` is only accepted in capture mode and must be a
/// PNG inside the capture folder; otherwise the file goes to `Pictures\Leveret`.
/// Returns the saved path.
#[tauri::command]
async fn capture_screen(
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
fn exit_app(app: AppHandle, code: i32) {
    app.exit(code);
}

fn create_main_window(app: &tauri::App, capture: bool) -> tauri::Result<()> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let options = capture::parse_args(std::env::args());
    let capturing = options.capture.is_some();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(options)
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let config_dir = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
            app.manage(RepoStore::load(&config_dir));
            create_main_window(app, capturing)?;
            Ok(())
        })
        .on_page_load(move |webview, payload| {
            if !capturing && payload.event() == PageLoadEvent::Finished {
                let _ = webview.window().show();
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_repos,
            add_repo,
            remove_repo,
            select_repo,
            get_status,
            get_working_diff,
            commit,
            get_log,
            get_commit,
            get_commit_diff,
            github_state,
            github_start_sign_in,
            github_poll_sign_in,
            github_sign_in_with_token,
            github_sign_out,
            github_list_repos,
            open_github_url,
            default_clone_dir,
            clone_repo,
            startup_options,
            capture_screen,
            exit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Leveret");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unregistered_repositories_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = RepoStore::load(dir.path());
        assert!(matches!(store.require("C:\\Windows"), Err(AppError::RepoNotRegistered(_))));
        assert!(store.select(Some("C:\\Windows".into())).is_err());
    }
}
