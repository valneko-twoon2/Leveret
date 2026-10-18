//! Commands for the history: the commit log (and searching it), commit details,
//! file diffs of a commit and who last changed each line of a file.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::blame::Blame;
use crate::git::diff::FileDiff;
use crate::git::log::{CommitDetail, CommitFile, LogEntry, LogQuery, LogScope};
use crate::repos::RepoStore;
use crate::{git, security};

/// Upper bound for the number of log entries requested at once.
const MAX_LOG_LIMIT: u32 = 100_000;

/// The log with its graph, or (with a `query`) the matching commits.
#[tauri::command]
pub async fn get_log(
    store: State<'_, RepoStore>,
    repo: String,
    scope: LogScope,
    limit: u32,
    query: Option<LogQuery>,
) -> Result<Vec<LogEntry>> {
    let repo = store.require(&repo)?;
    let limit = limit.clamp(1, MAX_LOG_LIMIT);
    match query.filter(|q| !q.is_empty()) {
        Some(q) => {
            check_query(&q)?;
            blocking(move || git::log::search(&repo, scope, limit, &q)).await
        }
        None => blocking(move || git::log::get(&repo, scope, limit)).await,
    }
}

pub(crate) fn check_query(q: &LogQuery) -> Result<()> {
    for text in [&q.message, &q.author, &q.content].into_iter().flatten() {
        security::search_text(text)?;
    }
    for path in [&q.path, &q.file].into_iter().flatten() {
        security::relative_path(path)?;
    }
    Ok(())
}

/// Files that differ between two commits; their diffs come from `get_commit_diff`
/// with `parent` set to `base`.
#[tauri::command]
pub async fn compare_commits(
    store: State<'_, RepoStore>,
    repo: String,
    base: String,
    target: String,
) -> Result<Vec<CommitFile>> {
    let repo = store.require(&repo)?;
    security::object_id(&base)?;
    security::object_id(&target)?;
    blocking(move || git::log::compare(&repo, &base, &target)).await
}

/// `rev`: blame the file as it is in that commit; without it, the working tree file.
#[tauri::command]
pub async fn get_blame(
    store: State<'_, RepoStore>,
    repo: String,
    rev: Option<String>,
    path: String,
    ignore_whitespace: Option<bool>,
) -> Result<Blame> {
    let repo = store.require(&repo)?;
    if let Some(r) = &rev {
        security::object_id(r)?;
    }
    security::relative_path(&path)?;
    let ws = ignore_whitespace.unwrap_or(false);
    blocking(move || git::blame::blame(&repo, rev.as_deref(), &path, ws)).await
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
    ignore_whitespace: Option<bool>,
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
    let ws = ignore_whitespace.unwrap_or(false);
    blocking(move || git::diff::commit_file_with(&repo, &hash, parent.as_deref(), &path, orig_path.as_deref(), ws))
        .await
}
