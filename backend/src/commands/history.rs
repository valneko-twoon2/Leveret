//! Commands for the history: the commit log, commit details and file diffs of a commit.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::diff::FileDiff;
use crate::git::log::{CommitDetail, LogEntry, LogScope};
use crate::repos::RepoStore;
use crate::{git, security};

/// Upper bound for the number of log entries requested at once.
const MAX_LOG_LIMIT: u32 = 100_000;

#[tauri::command]
pub async fn get_log(store: State<'_, RepoStore>, repo: String, scope: LogScope, limit: u32) -> Result<Vec<LogEntry>> {
    let repo = store.require(&repo)?;
    let limit = limit.clamp(1, MAX_LOG_LIMIT);
    blocking(move || git::log::get(&repo, scope, limit)).await
}

#[tauri::command]
pub async fn get_commit(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<CommitDetail> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || git::log::detail(&repo, &hash)).await
}

#[tauri::command]
pub async fn get_commit_diff(
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
