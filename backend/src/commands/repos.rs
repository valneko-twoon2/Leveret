//! Commands for the repository list: adding, removing and selecting repositories.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use super::blocking;
use crate::error::{AppError, Result};
use crate::repos::{RepoEntry, RepoList, RepoStore};

#[tauri::command]
pub fn list_repos(store: State<'_, RepoStore>) -> RepoList {
    store.list()
}

#[tauri::command]
pub async fn add_repo(app: AppHandle, path: String) -> Result<RepoEntry> {
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err(AppError::InvalidInput(path.display().to_string()));
    }
    blocking(move || app.state::<RepoStore>().add(&path, true)).await
}

#[tauri::command]
pub async fn remove_repo(app: AppHandle, path: String) -> Result<()> {
    blocking(move || app.state::<RepoStore>().remove(&path)).await
}

#[tauri::command]
pub async fn select_repo(app: AppHandle, path: Option<String>) -> Result<()> {
    blocking(move || app.state::<RepoStore>().select(path)).await
}
