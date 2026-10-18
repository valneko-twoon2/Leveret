//! The image diff: a picture as it is in the working tree or in a commit.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::image::Image;
use crate::repos::RepoStore;
use crate::{git, security};

/// The picture at `path` in commit `rev`, or in the working tree without `rev`;
/// None when it doesn't exist there. Only picture formats are served (see `git::image`).
#[tauri::command]
pub async fn get_image(
    store: State<'_, RepoStore>,
    repo: String,
    rev: Option<String>,
    path: String,
) -> Result<Option<Image>> {
    let repo = store.require(&repo)?;
    if let Some(r) = &rev {
        security::object_id(r)?;
    }
    security::relative_path(&path)?;
    blocking(move || git::image::read(&repo, rev.as_deref(), &path)).await
}
