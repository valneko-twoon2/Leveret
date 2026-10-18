//! Commands for operations that can stop on conflicts (merge, rebase, revert,
//! cherry-pick) and for resolving, continuing, skipping or aborting them.

use tauri::State;

use super::blocking;
use crate::error::{AppError, Result};
use crate::git::operation::Outcome;
use crate::git::rebase::{PlanCommit, Step};
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
/// Marks files as resolved. Unless `force` is set, files that still contain
/// conflict markers are refused (`conflictMarkersRemain`, listing them).
pub async fn mark_resolved(store: State<'_, RepoStore>, repo: String, paths: Vec<String>, force: bool) -> Result<()> {
    let repo = store.require(&repo)?;
    for p in &paths {
        security::relative_path(p)?;
    }
    blocking(move || {
        let left = git::resolve::files_with_markers(&repo, &paths);
        if !force && !left.is_empty() {
            return Err(AppError::ConflictMarkersRemain(left.join("\n")));
        }
        git::operation::mark_resolved(&repo, &paths)
    })
    .await
}

/// Resolves conflicted files with one side's version (the current files go to
/// the Recycle Bin).
#[tauri::command]
pub async fn resolve_conflict(
    store: State<'_, RepoStore>,
    repo: String,
    paths: Vec<String>,
    side: git::resolve::Side,
) -> Result<()> {
    let repo = store.require(&repo)?;
    for p in &paths {
        security::relative_path(p)?;
    }
    blocking(move || git::resolve::take_side(&repo, &paths, side)).await
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

/// The commits after `base` (from the first commit without one), oldest first,
/// for the interactive rebase dialog.
#[tauri::command]
pub async fn rebase_plan(store: State<'_, RepoStore>, repo: String, base: Option<String>) -> Result<Vec<PlanCommit>> {
    let repo = store.require(&repo)?;
    if let Some(b) = &base {
        security::object_id(b)?;
    }
    blocking(move || git::rebase::plan(&repo, base.as_deref())).await
}

/// Reorders, rewords, squashes and drops the commits after `base`.
#[tauri::command]
pub async fn interactive_rebase(
    store: State<'_, RepoStore>,
    repo: String,
    base: Option<String>,
    steps: Vec<Step>,
) -> Result<Outcome> {
    let repo = store.require(&repo)?;
    if let Some(b) = &base {
        security::object_id(b)?;
    }
    for s in &steps {
        security::object_id(&s.hash)?;
        if let Some(m) = &s.message {
            security::commit_message(m)?;
        }
    }
    blocking(move || git::rebase::run(&repo, base.as_deref(), &steps)).await
}
