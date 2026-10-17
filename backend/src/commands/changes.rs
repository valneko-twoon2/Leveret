//! Commands for the working tree: status, diffs of changed files, committing,
//! discarding, ignoring, undoing the last commit and showing files in Explorer.

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use super::blocking;
use crate::error::{AppError, Result};
use crate::git::diff::FileDiff;
use crate::git::status::RepoStatus;
use crate::repos::RepoStore;
use crate::{git, security};

/// `untracked: false` leaves untracked files out (the user's "hide untracked" setting).
#[tauri::command]
pub async fn get_status(store: State<'_, RepoStore>, repo: String, untracked: Option<bool>) -> Result<RepoStatus> {
    let repo = store.require(&repo)?;
    blocking(move || git::status::get_with(&repo, untracked.unwrap_or(true))).await
}

#[tauri::command]
pub async fn get_working_diff(
    store: State<'_, RepoStore>,
    repo: String,
    path: String,
    orig_path: Option<String>,
    untracked: bool,
    ignore_whitespace: Option<bool>,
) -> Result<FileDiff> {
    let repo = store.require(&repo)?;
    security::relative_path(&path)?;
    if let Some(o) = &orig_path {
        security::relative_path(o)?;
    }
    let ws = ignore_whitespace.unwrap_or(false);
    blocking(move || git::diff::working_with(&repo, &path, orig_path.as_deref(), untracked, ws)).await
}

#[tauri::command]
pub async fn commit(store: State<'_, RepoStore>, repo: String, request: git::commit::CommitRequest) -> Result<String> {
    let repo = store.require(&repo)?;
    for p in &request.paths {
        security::relative_path(p)?;
    }
    blocking(move || git::commit::commit(&repo, &request)).await
}

/// Moves the files to the Recycle Bin and restores them from HEAD.
#[tauri::command]
pub async fn discard_changes(store: State<'_, RepoStore>, repo: String, paths: Vec<String>) -> Result<()> {
    let repo = store.require(&repo)?;
    for p in &paths {
        security::relative_path(p)?;
    }
    blocking(move || git::discard::discard(&repo, &paths)).await
}

/// Undoes the last (unpushed) commit, keeping its changes. Returns its message.
#[tauri::command]
pub async fn undo_last_commit(store: State<'_, RepoStore>, repo: String) -> Result<String> {
    let repo = store.require(&repo)?;
    blocking(move || git::commit::undo_last(&repo)).await
}

#[tauri::command]
pub async fn ignore_path(store: State<'_, RepoStore>, repo: String, path: String, by_extension: bool) -> Result<()> {
    let repo = store.require(&repo)?;
    security::relative_path(&path)?;
    blocking(move || git::commit::ignore(&repo, &path, by_extension)).await
}

/// Opens the repository folder in Explorer, or selects `path` in it. Files are
/// only ever revealed, never opened, so nothing in a repository gets executed.
#[tauri::command]
pub async fn show_in_explorer(app: AppHandle, repo: String, path: Option<String>) -> Result<()> {
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
