//! GitHub issues of a repository's github.com remote (REST API v3). Requests go
//! through `crate::github::request`, which adds the signed-in user's token.

use std::path::Path;

use serde_json::{Value, json};

use super::{Comment, Issue, IssueDetail, IssueState, IssueUpdate, Label, Source, StateFilter};
use crate::error::{AppError, Result};
use crate::git::Git;
use crate::github;
use crate::security;

/// Pages of 100 read at most when listing issues or comments.
const MAX_PAGES: u32 = 10;

/// `owner/name` of the repository's github.com remote: `origin` if it points to
/// GitHub, otherwise the first remote that does.
pub fn repo_slug(repo: &Path) -> Result<Option<String>> {
    let out = Git::new(repo)
        .args(["config", "--get-regexp", r"^remote\..*\.url$"])
        .ok_codes(&[0, 1])
        .run_string()?;
    let mut remotes: Vec<(&str, String)> = out
        .lines()
        .filter_map(|l| {
            let (key, url) = l.split_once(' ')?;
            let name = key.strip_prefix("remote.")?.strip_suffix(".url")?;
            Some((name, parse_slug(url.trim())?))
        })
        .collect();
    remotes.sort_by_key(|(name, _)| *name != "origin");
    Ok(remotes.into_iter().next().map(|(_, slug)| slug))
}

/// `owner/name` from a github.com remote URL (https, ssh or scp-like), or `None`.
pub fn parse_slug(url: &str) -> Option<String> {
    let path = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("ssh://"))
        .map(|rest| {
            // Drop `user@` and require the host to be github.com.
            let rest = rest
                .split_once('@')
                .map_or(rest, |(user, host)| if user.contains('/') { rest } else { host });
            rest.strip_prefix("github.com/")
        })
        .unwrap_or_else(|| url.strip_prefix("git@github.com:"))?;
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, name) = path.split_once('/')?;
    let ok = |s: &str| {
        !s.is_empty() && s != "." && s != ".." && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (ok(owner) && ok(name)).then(|| format!("{owner}/{name}"))
}

fn require_slug(repo: &Path) -> Result<String> {
    repo_slug(repo)?.ok_or(AppError::NoGithubRemote)
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(String::from)
}

fn time(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_str).and_then(parse_time).unwrap_or(0)
}

/// Parses GitHub's `2026-10-17T12:34:56Z` into Unix seconds.
pub fn parse_time(s: &str) -> Option<i64> {
    let s = s.strip_suffix('Z')?;
    let (date, clock) = s.split_once('T')?;
    let mut d = date.splitn(3, '-').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let mut c = clock.splitn(3, ':').map(|p| p.parse::<i64>().ok());
    let (hh, mm, ss) = (c.next()??, c.next()??, c.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return None;
    }
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

fn to_issue(v: &Value) -> Option<Issue> {
    let state = match v.get("state")?.as_str()? {
        "closed" => IssueState::Closed,
        _ => IssueState::Open,
    };
    let labels = v
        .get("labels")
        .and_then(Value::as_array)
        .map(|ls| {
            ls.iter()
                .filter_map(|l| {
                    Some(Label {
                        name: text(l, "name")?,
                        color: text(l, "color"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Issue {
        source: Source::Github,
        id: v.get("number")?.as_u64()?,
        title: text(v, "title")?,
        body: text(v, "body").unwrap_or_default(),
        state,
        labels,
        author: v.get("user").and_then(|u| text(u, "login")),
        created_at: time(v, "created_at"),
        updated_at: time(v, "updated_at"),
        comments: v.get("comments").and_then(Value::as_u64).unwrap_or(0) as u32,
        url: text(v, "html_url"),
        published_as: None,
    })
}

/// Reads one issue. The issues API also serves pull requests under the same
/// numbers; those (and missing numbers) are reported as not found, so nothing here
/// ever edits or comments on a pull request.
fn fetch_issue(slug: &str, number: u64) -> Result<Value> {
    let v = github::request(
        "GET",
        &format!("/repos/{slug}/issues/{number}"),
        "GET /repos/{repo}/issues/{n}",
        None,
    )
    .map_err(|e| match e {
        AppError::GithubApi(m) if m.starts_with("HTTP 404") || m.starts_with("HTTP 410") => {
            AppError::IssueNotFound(number)
        }
        e => e,
    })?;
    if v.get("pull_request").is_some() {
        return Err(AppError::IssueNotFound(number));
    }
    Ok(v)
}

fn to_comment(v: &Value) -> Option<Comment> {
    Some(Comment {
        id: v.get("id")?.as_u64()?,
        author: v.get("user").and_then(|u| text(u, "login")),
        body: text(v, "body").unwrap_or_default(),
        created_at: time(v, "created_at"),
    })
}

/// Reads up to `MAX_PAGES` pages of a list endpoint (`path` already has a query).
fn pages(path: &str, name: &str) -> Result<Vec<Value>> {
    let mut items = Vec::new();
    for page in 1..=MAX_PAGES {
        let v = github::request("GET", &format!("{path}&per_page=100&page={page}"), name, None)?;
        let batch = v.as_array().cloned().unwrap_or_default();
        let count = batch.len();
        items.extend(batch);
        if count < 100 {
            break;
        }
    }
    Ok(items)
}

/// Issues (not pull requests), most recently updated first.
pub fn list(repo: &Path, filter: StateFilter) -> Result<Vec<Issue>> {
    let slug = require_slug(repo)?;
    let state = match filter {
        StateFilter::Open => "open",
        StateFilter::Closed => "closed",
        StateFilter::All => "all",
    };
    let items = pages(
        &format!("/repos/{slug}/issues?state={state}&sort=updated"),
        "GET /repos/{repo}/issues",
    )?;
    Ok(items
        .iter()
        .filter(|v| v.get("pull_request").is_none())
        .filter_map(to_issue)
        .collect())
}

/// The issue and its comments, read in parallel.
pub fn get(repo: &Path, number: u64) -> Result<IssueDetail> {
    let slug = require_slug(repo)?;
    let (issue, comments) = std::thread::scope(|s| {
        let comments = s.spawn(|| {
            pages(
                &format!("/repos/{slug}/issues/{number}/comments?"),
                "GET /repos/{repo}/issues/{n}/comments",
            )
        });
        let issue = fetch_issue(&slug, number);
        (issue, crate::git::join(comments))
    });
    let issue = to_issue(&issue?).ok_or(AppError::IssueNotFound(number))?;
    Ok(IssueDetail {
        issue,
        comments: comments?.iter().filter_map(to_comment).collect(),
    })
}

pub fn create(repo: &Path, title: &str, body: &str) -> Result<Issue> {
    security::issue_title(title)?;
    security::issue_text(body)?;
    let slug = require_slug(repo)?;
    let v = github::request(
        "POST",
        &format!("/repos/{slug}/issues"),
        "POST /repos/{repo}/issues",
        Some(&json!({ "title": title.trim(), "body": body })),
    )?;
    to_issue(&v).ok_or_else(|| AppError::GithubApi("unexpected issue response".into()))
}

pub fn update(repo: &Path, number: u64, change: &IssueUpdate) -> Result<Issue> {
    change.validate()?;
    let slug = require_slug(repo)?;
    let mut fields = serde_json::Map::new();
    if let Some(t) = &change.title {
        fields.insert("title".into(), json!(t.trim()));
    }
    if let Some(b) = &change.body {
        fields.insert("body".into(), json!(b));
    }
    if let Some(s) = change.state {
        fields.insert(
            "state".into(),
            json!(if s == IssueState::Closed { "closed" } else { "open" }),
        );
    }
    fetch_issue(&slug, number)?;
    let v = github::request(
        "PATCH",
        &format!("/repos/{slug}/issues/{number}"),
        "PATCH /repos/{repo}/issues/{n}",
        Some(&Value::Object(fields)),
    )?;
    to_issue(&v).ok_or_else(|| AppError::GithubApi("unexpected issue response".into()))
}

pub fn comment(repo: &Path, number: u64, body: &str) -> Result<Comment> {
    security::issue_comment(body)?;
    let slug = require_slug(repo)?;
    fetch_issue(&slug, number)?;
    let v = github::request(
        "POST",
        &format!("/repos/{slug}/issues/{number}/comments"),
        "POST /repos/{repo}/issues/{n}/comments",
        Some(&json!({ "body": body })),
    )?;
    to_comment(&v).ok_or_else(|| AppError::GithubApi("unexpected comment response".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_from_remote_urls() {
        for (url, slug) in [
            ("https://github.com/octo/leveret.git", Some("octo/leveret")),
            ("https://github.com/octo/leveret", Some("octo/leveret")),
            ("https://user@github.com/octo/leveret/", Some("octo/leveret")),
            ("git@github.com:octo/my.repo.git", Some("octo/my.repo")),
            ("ssh://git@github.com/octo/leveret.git", Some("octo/leveret")),
            ("https://gitlab.com/octo/leveret.git", None),
            ("https://github.com.evil.com/octo/leveret", None),
            ("https://github.com/octo", None),
            ("https://github.com/octo/../x", None),
            ("C:\\repos\\leveret", None),
        ] {
            assert_eq!(parse_slug(url).as_deref(), slug, "{url}");
        }
    }

    #[test]
    fn slug_prefers_origin() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        let git = |args: &[&str]| Git::new(r).args(args).run().unwrap();
        git(&["init", "-q"]);
        assert_eq!(repo_slug(r).unwrap(), None);
        git(&["remote", "add", "fork", "https://github.com/me/leveret.git"]);
        git(&["remote", "add", "origin", "git@github.com:octo/leveret.git"]);
        assert_eq!(repo_slug(r).unwrap().as_deref(), Some("octo/leveret"));
    }

    #[test]
    fn parses_times() {
        assert_eq!(parse_time("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_time("2026-10-17T12:34:56Z"), Some(1_792_240_496));
        assert_eq!(parse_time("2024-02-29T00:00:00Z"), Some(1_709_164_800));
        assert_eq!(parse_time("2026-13-01T00:00:00Z"), None);
        assert_eq!(parse_time("yesterday"), None);
    }

    #[test]
    fn maps_issues_and_skips_nothing_else() {
        let v: Value = serde_json::from_str(
            r#"{"number":7,"title":"Crash","body":null,"state":"closed","comments":2,
                "labels":[{"name":"bug","color":"d73a4a"}],"user":{"login":"octo"},
                "created_at":"2026-10-17T00:00:00Z","updated_at":"2026-10-17T01:00:00Z",
                "html_url":"https://github.com/octo/leveret/issues/7"}"#,
        )
        .unwrap();
        let i = to_issue(&v).unwrap();
        assert_eq!(
            (i.id, i.body.as_str(), i.state, i.comments),
            (7, "", IssueState::Closed, 2)
        );
        assert_eq!(i.labels[0].color.as_deref(), Some("d73a4a"));
        assert_eq!(i.updated_at - i.created_at, 3600);
    }
}
