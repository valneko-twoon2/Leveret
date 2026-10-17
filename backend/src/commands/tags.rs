//! Commands for tags: listing, creating, deleting and pushing them.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::git::tag::Tag;
use crate::repos::RepoStore;
use crate::{git, github, security};

#[tauri::command]
pub async fn list_tags(store: State<'_, RepoStore>, repo: String) -> Result<Vec<Tag>> {
    let repo = store.require(&repo)?;
    blocking(move || git::tag::list(&repo)).await
}

/// Creates a tag at a commit; annotated when `message` isn't empty.
#[tauri::command]
pub async fn create_tag(
    store: State<'_, RepoStore>,
    repo: String,
    name: String,
    target: String,
    message: String,
) -> Result<()> {
    let repo = store.require(&repo)?;
    security::tag_name(&name)?;
    security::object_id(&target)?;
    security::tag_message(&message)?;
    blocking(move || git::tag::create(&repo, &name, &target, &message)).await
}

/// Deletes a local tag, and with `remote` the tag on the default remote first.
#[tauri::command]
pub async fn delete_tag(store: State<'_, RepoStore>, repo: String, name: String, remote: bool) -> Result<()> {
    let repo = store.require(&repo)?;
    security::tag_name(&name)?;
    blocking(move || {
        if remote {
            git::tag::delete_remote(&repo, github::git_auth_env_for_repo(&repo), &name)?;
        }
        git::tag::delete(&repo, &name)
    })
    .await
}

#[tauri::command]
pub async fn push_tag(store: State<'_, RepoStore>, repo: String, name: String) -> Result<()> {
    let repo = store.require(&repo)?;
    security::tag_name(&name)?;
    blocking(move || git::tag::push(&repo, github::git_auth_env_for_repo(&repo), &name)).await
}
