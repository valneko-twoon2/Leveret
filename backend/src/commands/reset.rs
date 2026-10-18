//! Commands that rewrite: resetting the current branch to a commit, and
//! restoring one file from a commit (its version, or undoing the commit's change).

use serde::Deserialize;
use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::reset::{ResetInfo, ResetMode};
use crate::repos::RepoStore;
use crate::{git, security};

/// What resetting to `target` would do (shown before asking).
#[tauri::command]
pub async fn reset_info(store: State<'_, RepoStore>, repo: String, target: String) -> Result<ResetInfo> {
    let repo = store.require(&repo)?;
    security::object_id(&target)?;
    blocking(move || git::reset::reset_info(&repo, &target)).await
}

#[tauri::command]
pub async fn reset_branch(store: State<'_, RepoStore>, repo: String, target: String, mode: ResetMode) -> Result<()> {
    let repo = store.require(&repo)?;
    security::object_id(&target)?;
    blocking(move || git::reset::reset(&repo, &target, mode)).await
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreMode {
    /// The file as it is in the commit.
    Version,
    /// The current file without what the commit changed in it.
    Undo,
}

/// Changes one working tree file from a commit; the current file goes to the Recycle Bin.
#[tauri::command]
pub async fn restore_file(
    store: State<'_, RepoStore>,
    repo: String,
    hash: String,
    path: String,
    orig_path: Option<String>,
    mode: RestoreMode,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    security::relative_path(&path)?;
    if let Some(o) = &orig_path {
        security::relative_path(o)?;
    }
    blocking(move || match mode {
        RestoreMode::Version => git::reset::restore_file(&repo, &hash, &path),
        RestoreMode::Undo => git::reset::undo_file_change(&repo, &hash, &path, orig_path.as_deref()),
    })
    .await
}
