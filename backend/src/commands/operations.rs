//! Commands for operations that can stop on conflicts (merge, rebase, revert,
//! cherry-pick) and for resolving, continuing, skipping or aborting them.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::operation::Outcome;
use crate::repos::RepoStore;
use crate::{git, security};

#[tauri::command]
pub async fn revert_commit(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || {
        let is_merge = git::log::detail(&repo, &hash)?.parents.len() > 1;
        git::operation::revert(&repo, &hash, is_merge)
    })
    .await
}

#[tauri::command]
pub async fn rebase_branch(store: State<'_, RepoStore>, repo: String, onto: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::branch_ref(&onto)?;
    blocking(move || git::operation::rebase(&repo, &onto)).await
}

#[tauri::command]
pub async fn merge_branch(store: State<'_, RepoStore>, repo: String, branch: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::branch_ref(&branch)?;
    blocking(move || git::operation::merge(&repo, &branch)).await
}

#[tauri::command]
pub async fn mark_resolved(store: State<'_, RepoStore>, repo: String, paths: Vec<String>) -> Result<()> {
    let repo = store.require(&repo)?;
    for p in &paths {
        security::relative_path(p)?;
    }
    blocking(move || git::operation::mark_resolved(&repo, &paths)).await
}

#[tauri::command]
pub async fn continue_operation(store: State<'_, RepoStore>, repo: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    blocking(move || git::operation::continue_(&repo)).await
}

#[tauri::command]
pub async fn skip_operation(store: State<'_, RepoStore>, repo: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    blocking(move || git::operation::skip(&repo)).await
}

#[tauri::command]
pub async fn abort_operation(store: State<'_, RepoStore>, repo: String) -> Result<()> {
    let repo = store.require(&repo)?;
    blocking(move || git::operation::abort(&repo)).await
}

#[tauri::command]
pub async fn cherry_pick(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || {
        let is_merge = git::log::detail(&repo, &hash)?.parents.len() > 1;
        git::operation::cherry_pick(&repo, &hash, is_merge)
    })
    .await
}
