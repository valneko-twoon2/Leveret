//! Commands for stashes: listing, saving, restoring and deleting them.

use tauri::State;

use super::blocking;
use crate::error::{AppError, Result};
use crate::git;
use crate::git::operation::Outcome;
use crate::repos::RepoStore;

#[tauri::command]
pub async fn list_stashes(store: State<'_, RepoStore>, repo: String) -> Result<Vec<git::stash::StashEntry>> {
    let repo = store.require(&repo)?;
    blocking(move || git::stash::list(&repo)).await
}

#[tauri::command]
pub async fn stash_save(store: State<'_, RepoStore>, repo: String, message: String) -> Result<()> {
    let repo = store.require(&repo)?;
    if message.len() > 1000 || message.chars().any(char::is_control) {
        return Err(AppError::InvalidInput("stash message".into()));
    }
    blocking(move || git::stash::save(&repo, &message)).await
}

#[tauri::command]
pub async fn stash_pop(store: State<'_, RepoStore>, repo: String, index: u32) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    blocking(move || git::stash::pop(&repo, index)).await
}

#[tauri::command]
pub async fn stash_drop(store: State<'_, RepoStore>, repo: String, index: u32) -> Result<()> {
    let repo = store.require(&repo)?;
    blocking(move || git::stash::drop(&repo, index)).await
}
