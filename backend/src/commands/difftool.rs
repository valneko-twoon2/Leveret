//! Commands for opening diffs in WinMerge.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::repos::RepoStore;
use crate::{difftool, security};

/// Whether WinMerge is installed (the UI only offers it then).
#[tauri::command]
pub async fn get_diff_tool() -> Result<bool> {
    blocking(|| Ok(difftool::find().is_some())).await
}

/// Opens the uncommitted changes of a file (a 3-way merge for conflicts).
#[tauri::command]
pub async fn open_working_in_diff_tool(
    store: State<'_, RepoStore>,
    repo: String,
    path: String,
    orig_path: Option<String>,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::relative_path(&path)?;
    if let Some(o) = &orig_path {
        security::relative_path(o)?;
    }
    blocking(move || difftool::open_working(&repo, &path, orig_path.as_deref())).await
}

/// Opens one file of a commit against its first parent.
#[tauri::command]
pub async fn open_commit_in_diff_tool(
    store: State<'_, RepoStore>,
    repo: String,
    hash: String,
    parent: Option<String>,
    path: String,
    orig_path: Option<String>,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    if let Some(p) = &parent {
        security::object_id(p)?;
    }
    security::relative_path(&path)?;
    if let Some(o) = &orig_path {
        security::relative_path(o)?;
    }
    blocking(move || difftool::open_commit(&repo, &hash, parent.as_deref(), &path, orig_path.as_deref())).await
}
