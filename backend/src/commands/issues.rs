//! Commands for issues: local issues and the GitHub issues of the repository's
//! github.com remote, with optional publishing of local issues to GitHub.

use tauri::State;

use super::blocking;
use crate::error::Result;
use crate::issues::{self, Comment, Issue, IssueDetail, IssueSources, IssueUpdate, Source, StateFilter};
use crate::repos::RepoStore;
use crate::{github, security};

/// Whether the repository has GitHub issues and whether the user is signed in.
#[tauri::command]
pub async fn issue_sources(store: State<'_, RepoStore>, repo: String) -> Result<IssueSources> {
    let repo = store.require(&repo)?;
    blocking(move || {
        Ok(IssueSources {
            github_repo: issues::github::repo_slug(&repo)?,
            signed_in: github::signed_in(),
        })
    })
    .await
}

#[tauri::command]
pub async fn list_issues(
    store: State<'_, RepoStore>,
    repo: String,
    source: Source,
    state: StateFilter,
) -> Result<Vec<Issue>> {
    let repo = store.require(&repo)?;
    blocking(move || match source {
        Source::Local => issues::local::list(&repo, state),
        Source::Github => issues::github::list(&repo, state),
    })
    .await
}

#[tauri::command]
pub async fn get_issue(store: State<'_, RepoStore>, repo: String, source: Source, id: u64) -> Result<IssueDetail> {
    let repo = store.require(&repo)?;
    security::issue_id(id)?;
    blocking(move || match source {
        Source::Local => issues::local::get(&repo, id),
        Source::Github => issues::github::get(&repo, id),
    })
    .await
}

#[tauri::command]
pub async fn create_issue(
    store: State<'_, RepoStore>,
    repo: String,
    source: Source,
    title: String,
    body: String,
) -> Result<Issue> {
    let repo = store.require(&repo)?;
    security::issue_title(&title)?;
    security::issue_text(&body)?;
    blocking(move || match source {
        Source::Local => issues::local::create(&repo, &title, &body),
        Source::Github => issues::github::create(&repo, &title, &body),
    })
    .await
}

#[tauri::command]
pub async fn update_issue(
    store: State<'_, RepoStore>,
    repo: String,
    source: Source,
    id: u64,
    change: IssueUpdate,
) -> Result<Issue> {
    let repo = store.require(&repo)?;
    security::issue_id(id)?;
    change.validate()?;
    blocking(move || match source {
        Source::Local => issues::local::update(&repo, id, &change),
        Source::Github => issues::github::update(&repo, id, &change),
    })
    .await
}

#[tauri::command]
pub async fn comment_issue(
    store: State<'_, RepoStore>,
    repo: String,
    source: Source,
    id: u64,
    body: String,
) -> Result<Comment> {
    let repo = store.require(&repo)?;
    security::issue_id(id)?;
    security::issue_comment(&body)?;
    blocking(move || match source {
        Source::Local => issues::local::comment(&repo, id, &body),
        Source::Github => issues::github::comment(&repo, id, &body),
    })
    .await
}

/// Deletes a local issue (GitHub issues can only be closed).
#[tauri::command]
pub async fn delete_issue(store: State<'_, RepoStore>, repo: String, id: u64) -> Result<()> {
    let repo = store.require(&repo)?;
    security::issue_id(id)?;
    blocking(move || issues::local::delete(&repo, id)).await
}

/// Creates a local issue on GitHub (with its comments) and removes the local copy.
#[tauri::command]
pub async fn publish_issue(store: State<'_, RepoStore>, repo: String, id: u64) -> Result<Issue> {
    let repo = store.require(&repo)?;
    security::issue_id(id)?;
    blocking(move || issues::publish(&repo, id)).await
}
