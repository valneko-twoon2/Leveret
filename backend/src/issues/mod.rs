//! Issues: GitHub issues of a repository's github.com remote, and local issues kept
//! next to the repository (`<git common dir>/leveret/issues.json`, never committed).
//! A local issue can be published to GitHub later (optional): it is created there,
//! its comments are posted in order, and the local copy is removed.
//!
//! Both kinds share one shape ([`Issue`]), so the app and agents treat them alike.

pub mod github;
pub mod local;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::security;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Local,
    Github,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueState {
    Open,
    Closed,
}

/// Which issues to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StateFilter {
    Open,
    Closed,
    All,
}

impl StateFilter {
    pub fn matches(self, state: IssueState) -> bool {
        match self {
            StateFilter::Open => state == IssueState::Open,
            StateFilter::Closed => state == IssueState::Closed,
            StateFilter::All => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub name: String,
    /// Hex color without `#` (GitHub labels only).
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub source: Source,
    /// Local id, or the GitHub issue number.
    pub id: u64,
    pub title: String,
    pub body: String,
    pub state: IssueState,
    pub labels: Vec<Label>,
    /// GitHub login; `None` for local issues.
    pub author: Option<String>,
    /// Unix seconds.
    pub created_at: i64,
    pub updated_at: i64,
    pub comments: u32,
    /// The issue page on github.com.
    pub url: Option<String>,
    /// Local issues whose publishing stopped halfway: the GitHub issue already
    /// created (publishing again continues with it).
    pub published_as: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: u64,
    pub author: Option<String>,
    pub body: String,
    /// Unix seconds.
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueDetail {
    pub issue: Issue,
    pub comments: Vec<Comment>,
}

/// Fields to change; `None` leaves a field as it is.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueUpdate {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<IssueState>,
}

impl IssueUpdate {
    /// Validates the fields that are set (the webview and agents are untrusted).
    pub fn validate(&self) -> Result<()> {
        if let Some(t) = &self.title {
            security::issue_title(t)?;
        }
        if let Some(b) = &self.body {
            security::issue_text(b)?;
        }
        Ok(())
    }
}

/// What the app shows for a repository: whether GitHub issues are available, and
/// why not when they aren't.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueSources {
    /// `owner/name` of the github.com remote, if any.
    pub github_repo: Option<String>,
    pub signed_in: bool,
}

/// Publishes a local issue to the repository's GitHub repository: creates it with
/// its comments (closed if it was closed locally) and removes the local copy.
/// See [`local::publish`] for how a failure halfway is resumed.
pub fn publish(repo: &Path, id: u64) -> Result<Issue> {
    if github::repo_slug(repo)?.is_none() {
        return Err(AppError::NoGithubRemote);
    }
    if !crate::github::signed_in() {
        return Err(AppError::GithubNotSignedIn);
    }
    local::publish(repo, id, &mut GithubRemote { repo })
}

struct GithubRemote<'a> {
    repo: &'a Path,
}

impl local::Remote for GithubRemote<'_> {
    fn create(&mut self, title: &str, body: &str) -> Result<Issue> {
        github::create(self.repo, title, body)
    }

    fn get(&mut self, number: u64) -> Result<Issue> {
        github::get(self.repo, number).map(|d| d.issue)
    }

    fn comment(&mut self, number: u64, body: &str) -> Result<()> {
        github::comment(self.repo, number, body).map(|_| ())
    }

    fn close(&mut self, number: u64) -> Result<()> {
        let close = IssueUpdate {
            state: Some(IssueState::Closed),
            ..Default::default()
        };
        github::update(self.repo, number, &close).map(|_| ())
    }
}

pub(crate) fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
