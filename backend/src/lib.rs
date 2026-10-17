mod agent;
mod capture;
mod crash;
mod error;
mod git;
mod github;
mod lockdown;
mod repos;
mod security;
mod settings;

use std::path::PathBuf;

use serde::Serialize;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use capture::StartupOptions;
use error::{AppError, Result};
use git::branch::Branch;
use git::diff::FileDiff;
use git::log::{CommitDetail, LogEntry, LogScope};
use git::operation::Outcome;
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
    blocking(move || app.state::<RepoStore>().add(&path, true)).await
}

#[tauri::command]
async fn remove_repo(app: AppHandle, path: String) -> Result<()> {
    blocking(move || app.state::<RepoStore>().remove(&path)).await
}

#[tauri::command]
async fn select_repo(app: AppHandle, path: Option<String>) -> Result<()> {
    blocking(move || app.state::<RepoStore>().select(path)).await
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

// --- discard, revert, rebase -----------------------------------------------

/// Moves the files to the Recycle Bin and restores them from HEAD.
#[tauri::command]
async fn discard_changes(store: State<'_, RepoStore>, repo: String, paths: Vec<String>) -> Result<()> {
    let repo = store.require(&repo)?;
    for p in &paths {
        security::relative_path(p)?;
    }
    blocking(move || git::discard::discard(&repo, &paths)).await
}

#[tauri::command]
async fn revert_commit(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || {
        let is_merge = git::log::detail(&repo, &hash)?.parents.len() > 1;
        git::operation::revert(&repo, &hash, is_merge)
    })
    .await
}

#[tauri::command]
async fn rebase_branch(store: State<'_, RepoStore>, repo: String, onto: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::branch_ref(&onto)?;
    blocking(move || git::operation::rebase(&repo, &onto)).await
}

#[tauri::command]
async fn merge_branch(store: State<'_, RepoStore>, repo: String, branch: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::branch_ref(&branch)?;
    blocking(move || git::operation::merge(&repo, &branch)).await
}

#[tauri::command]
async fn mark_resolved(store: State<'_, RepoStore>, repo: String, paths: Vec<String>) -> Result<()> {
    let repo = store.require(&repo)?;
    for p in &paths {
        security::relative_path(p)?;
    }
    blocking(move || git::operation::mark_resolved(&repo, &paths)).await
}

#[tauri::command]
async fn continue_operation(store: State<'_, RepoStore>, repo: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    blocking(move || git::operation::continue_(&repo)).await
}

#[tauri::command]
async fn skip_operation(store: State<'_, RepoStore>, repo: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    blocking(move || git::operation::skip(&repo)).await
}

#[tauri::command]
async fn abort_operation(store: State<'_, RepoStore>, repo: String) -> Result<()> {
    let repo = store.require(&repo)?;
    blocking(move || git::operation::abort(&repo)).await
}

#[tauri::command]
async fn cherry_pick(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || {
        let is_merge = git::log::detail(&repo, &hash)?.parents.len() > 1;
        git::operation::cherry_pick(&repo, &hash, is_merge)
    })
    .await
}

/// Undoes the last (unpushed) commit, keeping its changes. Returns its message.
#[tauri::command]
async fn undo_last_commit(store: State<'_, RepoStore>, repo: String) -> Result<String> {
    let repo = store.require(&repo)?;
    blocking(move || git::commit::undo_last(&repo)).await
}

#[tauri::command]
async fn ignore_path(store: State<'_, RepoStore>, repo: String, path: String, by_extension: bool) -> Result<()> {
    let repo = store.require(&repo)?;
    security::relative_path(&path)?;
    blocking(move || git::commit::ignore(&repo, &path, by_extension)).await
}

/// Opens the repository folder in Explorer, or selects `path` in it. Files are
/// only ever revealed, never opened, so nothing in a repository gets executed.
#[tauri::command]
async fn show_in_explorer(app: AppHandle, repo: String, path: Option<String>) -> Result<()> {
    let root = app.state::<RepoStore>().require(&repo)?;
    if let Some(p) = &path {
        security::relative_path(p)?;
    }
    blocking(move || {
        let opener = app.opener();
        let target = path.map(|p| root.join(p)).filter(|p| p.exists());
        match target {
            Some(t) => opener.reveal_item_in_dir(t),
            None => opener.open_path(root.display().to_string(), None::<&str>),
        }
        .map_err(|e| AppError::Other(e.to_string()))
    })
    .await
}

// --- stash ------------------------------------------------------------------

#[tauri::command]
async fn list_stashes(store: State<'_, RepoStore>, repo: String) -> Result<Vec<git::stash::StashEntry>> {
    let repo = store.require(&repo)?;
    blocking(move || git::stash::list(&repo)).await
}

#[tauri::command]
async fn stash_save(store: State<'_, RepoStore>, repo: String, message: String) -> Result<()> {
    let repo = store.require(&repo)?;
    if message.len() > 1000 || message.chars().any(char::is_control) {
        return Err(AppError::InvalidInput("stash message".into()));
    }
    blocking(move || git::stash::save(&repo, &message)).await
}

#[tauri::command]
async fn stash_pop(store: State<'_, RepoStore>, repo: String, index: u32) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    blocking(move || git::stash::pop(&repo, index)).await
}

#[tauri::command]
async fn stash_drop(store: State<'_, RepoStore>, repo: String, index: u32) -> Result<()> {
    let repo = store.require(&repo)?;
    blocking(move || git::stash::drop(&repo, index)).await
}

// --- branches ---------------------------------------------------------------

#[tauri::command]
async fn list_branches(store: State<'_, RepoStore>, repo: String) -> Result<Vec<Branch>> {
    let repo = store.require(&repo)?;
    blocking(move || git::branch::list(&repo)).await
}

#[tauri::command]
async fn create_branch(
    store: State<'_, RepoStore>,
    repo: String,
    name: String,
    start: Option<String>,
    checkout: bool,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_name(&name)?;
    if let Some(s) = &start {
        security::revision(s)?;
    }
    blocking(move || git::branch::create(&repo, &name, start.as_deref(), checkout)).await
}

#[tauri::command]
async fn switch_branch(store: State<'_, RepoStore>, repo: String, branch: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_ref(&branch)?;
    blocking(move || git::branch::switch(&repo, &branch)).await
}

#[tauri::command]
async fn checkout_commit(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || git::branch::checkout_detached(&repo, &hash)).await
}

#[tauri::command]
async fn delete_branch(store: State<'_, RepoStore>, repo: String, name: String, force: bool) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_name(&name)?;
    blocking(move || git::branch::delete(&repo, &name, force)).await
}

// --- remote -----------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum RemoteOp {
    Fetch,
    Pull,
    Push,
    Sync,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RemoteProgress {
    repo: String,
    operation: RemoteOp,
    #[serde(flatten)]
    progress: git::Progress,
}

/// Fetch, pull, push or sync. Progress is emitted as `remote-progress` events.
#[tauri::command]
async fn remote(
    store: State<'_, RepoStore>,
    window: WebviewWindow,
    repo: String,
    operation: RemoteOp,
) -> Result<Outcome> {
    let path = store.require(&repo)?;
    blocking(move || {
        let envs = github::git_auth_env_for_repo(&path);
        let report = |progress| {
            let _ = window.emit(
                "remote-progress",
                RemoteProgress {
                    repo: repo.clone(),
                    operation,
                    progress,
                },
            );
        };
        match operation {
            RemoteOp::Fetch => git::remote::fetch(&path, envs, report).map(|()| Outcome::Done),
            RemoteOp::Pull => git::remote::pull(&path, envs, report),
            RemoteOp::Push => git::remote::push(&path, envs, report).map(|()| Outcome::Done),
            RemoteOp::Sync => git::remote::sync(&path, envs, report),
        }
    })
    .await
}

// --- AI agents --------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentTool {
    name: &'static str,
    title: &'static str,
    level: agent::Level,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentInfo {
    /// Path of this executable, for the MCP client configuration (`<exe> --mcp`).
    exe_path: String,
    access: agent::Access,
    tools: Vec<AgentTool>,
    activity: Vec<agent::Activity>,
}

#[tauri::command]
async fn agent_info() -> Result<AgentInfo> {
    blocking(|| {
        let dir = settings::config_dir();
        let exe_path = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let tools = agent::tools::all()
            .into_iter()
            .map(|t| AgentTool {
                name: t.name,
                title: t.title,
                level: t.level,
            })
            .collect();
        Ok(AgentInfo {
            exe_path,
            access: settings::load(&dir).agent_access,
            tools,
            activity: agent::recent(&dir, 50),
        })
    })
    .await
}

#[tauri::command]
async fn set_agent_access(access: agent::Access) -> Result<()> {
    blocking(move || {
        let dir = settings::config_dir();
        let mut s = settings::load(&dir);
        s.agent_access = access;
        settings::save(&dir, &s)
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentCallResult {
    text: String,
    is_error: bool,
}

/// Runs an agent tool in-process (for agents running inside Leveret). Same access
/// checks and activity log as the MCP server.
#[tauri::command]
async fn agent_call(app: AppHandle, name: String, arguments: serde_json::Value) -> Result<AgentCallResult> {
    blocking(move || {
        let dir = settings::config_dir();
        let ctx = agent::tools::Context {
            store: &app.state::<RepoStore>(),
            config_dir: &dir,
            client: "Leveret",
        };
        Ok(match agent::tools::call(&ctx, &name, &arguments) {
            Ok(out) => AgentCallResult {
                text: out.into_text(),
                is_error: false,
            },
            Err(agent::tools::CallError::UnknownTool) => {
                return Err(AppError::InvalidInput(format!("unknown tool: {name}")));
            }
            Err(agent::tools::CallError::Denied(m) | agent::tools::CallError::Failed(m)) => AgentCallResult {
                text: m,
                is_error: true,
            },
        })
    })
    .await
}

/// Emits `agent-activity` with new entries whenever an agent (usually `leveret --mcp`
/// in another process) changes something, so the app can refresh and tell the user.
fn watch_agent_activity(app: AppHandle) {
    std::thread::spawn(move || {
        let dir = settings::config_dir();
        let file = agent::activity_file(&dir);
        let modified = |f: &std::path::Path| std::fs::metadata(f).and_then(|m| m.modified()).ok();
        let mut seen = modified(&file);
        let mut last = agent::recent(&dir, 1).last().map_or(0, |a| a.time);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let now = modified(&file);
            if now == seen {
                continue;
            }
            seen = now;
            let new = agent::since(&dir, last);
            if let Some(newest) = new.last() {
                last = newest.time;
                let _ = app.emit("agent-activity", new);
            }
        }
    });
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
    Ok(GithubState {
        account,
        device_flow: github::client_id().is_some(),
    })
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
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Other(e.to_string()))
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
async fn clone_repo(
    app: AppHandle,
    window: WebviewWindow,
    url: String,
    parent: String,
    name: String,
) -> Result<RepoEntry> {
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
        app.state::<RepoStore>().add(&dest, true)
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
    FrontendOptions {
        startup: options.inner().clone(),
        devtools: lockdown::devtools_allowed(),
    }
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
    crash::install_panic_hook();
    // `leveret --mcp`: serve the agent tools over stdio, without any window.
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        std::process::exit(agent::mcp::serve());
    }
    let options = capture::parse_args(std::env::args());
    let capturing = options.capture.is_some();

    let result = tauri::Builder::default()
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
            app.manage(RepoStore::load(&settings::config_dir()));
            create_main_window(app, capturing)?;
            if !capturing {
                watch_agent_activity(app.handle().clone());
            }
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
            discard_changes,
            revert_commit,
            rebase_branch,
            merge_branch,
            mark_resolved,
            continue_operation,
            skip_operation,
            abort_operation,
            cherry_pick,
            undo_last_commit,
            ignore_path,
            show_in_explorer,
            list_stashes,
            stash_save,
            stash_pop,
            stash_drop,
            list_branches,
            create_branch,
            switch_branch,
            checkout_commit,
            delete_branch,
            remote,
            agent_info,
            set_agent_access,
            agent_call,
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
        .run(tauri::generate_context!());
    if let Err(e) = result {
        crash::fatal(&format!("Leveret could not start.\n\n{e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unregistered_repositories_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = RepoStore::load(dir.path());
        assert!(matches!(
            store.require("C:\\Windows"),
            Err(AppError::RepoNotRegistered(_))
        ));
        assert!(store.select(Some("C:\\Windows".into())).is_err());
    }
}
