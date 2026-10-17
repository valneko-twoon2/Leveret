//! Commands for cloning a repository: the default folder and the clone itself.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use super::blocking;
use crate::error::{AppError, Result};
use crate::repos::{RepoEntry, RepoStore};
use crate::{git, github, security};

#[tauri::command]
pub fn default_clone_dir(app: AppHandle) -> Result<String> {
    let docs = app.path().document_dir().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(docs.join("GitHub").display().to_string())
}

/// Clones `url` into `<parent>\<name>` and registers it. Progress is emitted as
/// `clone-progress` events.
#[tauri::command]
pub async fn clone_repo(
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
