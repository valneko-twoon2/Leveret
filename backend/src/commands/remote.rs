//! Commands for remotes: fetch, pull, push and sync with progress events.

use serde::Serialize;
use tauri::{Emitter, State, WebviewWindow};

use super::blocking;
use crate::error::Result;
use crate::git::operation::Outcome;
use crate::repos::RepoStore;
use crate::{git, github};

#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RemoteOp {
    Fetch,
    Pull,
    Push,
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
            RemoteOp::Sync => git::remote::sync(&path, envs, report),
        }
    })
    .await
}
