//! Commands for remotes: fetch, pull, push and sync with progress events, and managing
//! the list of remotes. Changing a remote is not an agent tool: a different URL would
//! redirect where the code is pushed.

use serde::Serialize;
use tauri::{Emitter, State, WebviewWindow};

use super::blocking;
use crate::error::Result;
use crate::git::operation::Outcome;
use crate::git::remotes::Remote;
use crate::repos::RepoStore;
use crate::{git, github, security};

#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RemoteOp {
    Fetch,
    Pull,
    Push,
    /// Push with `--force-with-lease` (asks for confirmation in the app).
    ForcePush,
    Sync,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RemoteProgress {
    repo: String,
    operation: RemoteOp,
    #[serde(flatten)]
    progress: git::Progress,
}

/// Fetch, pull, push or sync. Progress is emitted as `remote-progress` events.
#[tauri::command]
pub async fn remote(
    store: State<'_, RepoStore>,
    window: WebviewWindow,
    repo: String,
    operation: RemoteOp,
) -> Result<Outcome> {
    let path = store.require(&repo)?;
    blocking(move || {
        let envs = github::git_auth_env_for_repo(&path);
        let report = |progress| {
            let _ = window.emit(
                "remote-progress",
                RemoteProgress {
                    repo: repo.clone(),
                    operation,
                    progress,
                },
            );
        };
        match operation {
            RemoteOp::Fetch => git::remote::fetch(&path, envs, report).map(|()| Outcome::Done),
            RemoteOp::Pull => git::remote::pull(&path, envs, report),
            RemoteOp::Push => git::remote::push(&path, envs, report).map(|()| Outcome::Done),
            RemoteOp::ForcePush => git::remote::force_push(&path, envs, report).map(|()| Outcome::Done),
            RemoteOp::Sync => git::remote::sync(&path, envs, report),
        }
    })
    .await
}

#[tauri::command]
pub async fn list_remotes(store: State<'_, RepoStore>, repo: String) -> Result<Vec<Remote>> {
    let repo = store.require(&repo)?;
    blocking(move || git::remotes::list(&repo)).await
}

#[tauri::command]
pub async fn add_remote(store: State<'_, RepoStore>, repo: String, name: String, url: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::remote_name(&name)?;
    security::clone_url(&url)?;
    blocking(move || git::remotes::add(&repo, &name, &url)).await
}

#[tauri::command]
pub async fn rename_remote(store: State<'_, RepoStore>, repo: String, name: String, new_name: String) -> Result<()> {
    let repo = store.require(&repo)?;
    // The old name may be any configured remote (names with `/` exist); git checks it.
    security::branch_name(&name)?;
    security::remote_name(&new_name)?;
    blocking(move || git::remotes::rename(&repo, &name, &new_name)).await
}

#[tauri::command]
pub async fn remove_remote(store: State<'_, RepoStore>, repo: String, name: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_name(&name)?;
    blocking(move || git::remotes::remove(&repo, &name)).await
}

/// Sets the fetch URL, or with `push` the push URL, of a remote.
#[tauri::command]
pub async fn set_remote_url(
    store: State<'_, RepoStore>,
    repo: String,
    name: String,
    url: String,
    push: bool,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::branch_name(&name)?;
    security::clone_url(&url)?;
    blocking(move || git::remotes::set_url(&repo, &name, &url, push)).await
}
