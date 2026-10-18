//! Commands for branches: listing, creating, switching and deleting them, and
//! checking out a commit.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::branch::Branch;
use crate::repos::RepoStore;
use crate::{git, github, security};

#[tauri::command]
pub async fn list_branches(store: State<'_, RepoStore>, repo: String) -> Result<Vec<Branch>> {
    let repo = store.require(&repo)?;
    blocking(move || git::branch::list(&repo)).await
}

#[tauri::command]
pub async fn create_branch(
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
pub async fn switch_branch(store: State<'_, RepoStore>, repo: String, branch: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_ref(&branch)?;
    blocking(move || git::branch::switch(&repo, &branch)).await
}

#[tauri::command]
pub async fn checkout_commit(store: State<'_, RepoStore>, repo: String, hash: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::object_id(&hash)?;
    blocking(move || git::branch::checkout_detached(&repo, &hash)).await
}

#[tauri::command]
pub async fn delete_branch(store: State<'_, RepoStore>, repo: String, name: String, force: bool) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_name(&name)?;
    blocking(move || git::branch::delete(&repo, &name, force)).await
}

#[tauri::command]
pub async fn rename_branch(store: State<'_, RepoStore>, repo: String, name: String, new_name: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_name(&name)?;
    security::branch_name(&new_name)?;
    blocking(move || git::branch::rename(&repo, &name, &new_name)).await
}

/// Sets the upstream of a local branch to a remote branch (a full ref), or removes it.
#[tauri::command]
pub async fn set_upstream(
    store: State<'_, RepoStore>,
    repo: String,
    name: String,
    upstream: Option<String>,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_name(&name)?;
    if let Some(u) = &upstream {
        security::remote_branch_ref(u)?;
    }
    blocking(move || git::branch::set_upstream(&repo, &name, upstream.as_deref())).await
}

/// Deletes a branch on its remote (given as `refs/remotes/<remote>/<branch>`).
#[tauri::command]
pub async fn delete_remote_branch(store: State<'_, RepoStore>, repo: String, branch: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::remote_branch_ref(&branch)?;
    blocking(move || git::branch::delete_remote(&repo, github::git_auth_env_for_repo(&repo), &branch)).await
}
