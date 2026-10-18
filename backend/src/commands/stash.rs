//! Commands for stashes: listing, saving (everything or some files), looking
//! inside, restoring (keeping or dropping the stash) and deleting them.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::operation::Outcome;
use crate::git::stash::{StashDetail, StashEntry};
use crate::repos::RepoStore;
use crate::{git, security};

#[tauri::command]
pub async fn list_stashes(store: State<'_, RepoStore>, repo: String) -> Result<Vec<StashEntry>> {
    let repo = store.require(&repo)?;
    blocking(move || git::stash::list(&repo)).await
}

/// Stashes every change, or only `paths` (as listed by status; renames with both paths).
#[tauri::command]
pub async fn stash_save(
    store: State<'_, RepoStore>,
    repo: String,
    message: String,
    untracked: Option<bool>,
    paths: Option<Vec<String>>,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::stash_message(&message)?;
    let paths = paths.unwrap_or_default();
    for p in &paths {
        security::relative_path(p)?;
    }
    blocking(move || git::stash::save(&repo, &message, untracked.unwrap_or(true), &paths)).await
}

/// The files of a stash; their diffs come from `get_commit_diff`.
#[tauri::command]
pub async fn get_stash(store: State<'_, RepoStore>, repo: String, oid: String) -> Result<StashDetail> {
    let repo = store.require(&repo)?;
    security::object_id(&oid)?;
    blocking(move || git::stash::detail(&repo, &oid)).await
}

fn check_oid(oid: &Option<String>) -> Result<()> {
    match oid {
        Some(o) => security::object_id(o).map(|_| ()),
        None => Ok(()),
    }
}

#[tauri::command]
pub async fn stash_pop(store: State<'_, RepoStore>, repo: String, index: u32, oid: Option<String>) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    check_oid(&oid)?;
    blocking(move || git::stash::pop(&repo, index, oid.as_deref())).await
}

#[tauri::command]
pub async fn stash_apply(
    store: State<'_, RepoStore>,
    repo: String,
    index: u32,
    oid: Option<String>,
) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    check_oid(&oid)?;
    blocking(move || git::stash::apply(&repo, index, oid.as_deref())).await
}

#[tauri::command]
pub async fn stash_drop(store: State<'_, RepoStore>, repo: String, index: u32, oid: Option<String>) -> Result<()> {
    let repo = store.require(&repo)?;
    check_oid(&oid)?;
    blocking(move || git::stash::drop(&repo, index, oid.as_deref())).await
}
