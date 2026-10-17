//! Tauri commands: what the webview can ask the backend to do.
//!
//! The webview is untrusted, so every command validates its arguments
//! (registered repositories, `security::*` validators) before they reach the
//! file system or git. Long work runs in [`blocking`].
//!
//! Some modules also hold the setup code their commands depend on: the main
//! window (`app`) and the agent activity watcher (`agent`).

pub(crate) mod agent;
pub(crate) mod app;
pub(crate) mod branches;
pub(crate) mod changes;
pub(crate) mod clone;
pub(crate) mod github;
pub(crate) mod history;
pub(crate) mod issues;
pub(crate) mod operations;
pub(crate) mod perf;
pub(crate) mod remote;
pub(crate) mod repos;
pub(crate) mod stash;

use crate::error::{AppError, Result};

/// Runs blocking work (git, network) off the async runtime's worker threads.
async fn blocking<T, F>(f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Other(e.to_string()))?
}
