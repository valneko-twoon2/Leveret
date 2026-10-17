//! Issue tools: local issues and the GitHub issues of a repository's github.com
//! remote, and publishing local issues to GitHub.

use serde_json::{Value, json};

use super::args::{opt_str, opt_u64, repo, str_arg};
use super::render::iso8601;
use super::schema::{integer, object, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::{AppError, Result};
use crate::issues::{self, Issue, IssueState, IssueUpdate, Source, StateFilter};
use crate::security;

fn source_prop(default: &str) -> Value {
    json!({ "type": "string", "enum": ["local", "github"],
            "description": format!("local (kept in Leveret, not on GitHub) or github. Default {default}.") })
}

fn id_prop() -> Value {
    integer("Local issue id or GitHub issue number.", 1, 1_000_000_000_000)
}

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "list_issues",
            "List issues",
            Read,
            true,
            "Local issues and, when the repository has a github.com remote, its GitHub issues (not pull requests), \
             most recently updated first.",
            object(
                json!({
                    "repo": repo_prop(),
                    "source": { "type": "string", "enum": ["local", "github", "all"], "description": "Default all." },
                    "state": { "type": "string", "enum": ["open", "closed", "all"], "description": "Default open." },
                }),
                &["repo"],
            ),
        ),
        tool(
            "get_issue",
            "Show issue",
            Read,
            true,
            "An issue's title, body, labels and comments.",
            object(
                json!({ "repo": repo_prop(), "source": source_prop("github"), "id": id_prop() }),
                &["repo", "id"],
            ),
        ),
        tool(
            "create_issue",
            "Create issue",
            Write,
            true,
            "Creates a local issue (default; can be published to GitHub later with publish_issue) or a GitHub issue.",
            object(
                json!({
                    "repo": repo_prop(),
                    "title": string("Title (1-256 characters)."),
                    "body": string("Markdown body (optional)."),
                    "source": source_prop("local"),
                }),
                &["repo", "title"],
            ),
        ),
        tool(
            "update_issue",
            "Edit issue",
            Write,
            true,
            "Changes an issue's title or body, or closes / reopens it. Omitted fields stay as they are.",
            object(
                json!({
                    "repo": repo_prop(),
                    "source": source_prop("github"),
                    "id": id_prop(),
                    "title": string("New title."),
                    "body": string("New Markdown body."),
                    "state": { "type": "string", "enum": ["open", "closed"] },
                }),
                &["repo", "id"],
            ),
        ),
        tool(
            "comment_issue",
            "Comment on issue",
            Write,
            true,
            "Adds a comment to an issue.",
            object(
                json!({ "repo": repo_prop(), "source": source_prop("github"), "id": id_prop(), "body": string("Markdown comment.") }),
                &["repo", "id", "body"],
            ),
        ),
        tool(
            "publish_issue",
            "Publish issue to GitHub",
            Write,
            true,
            "Creates a local issue on GitHub (with its comments), then removes the local copy. Returns the GitHub issue. \
             Needs a github.com remote and the user signed in to GitHub in Leveret.",
            object(json!({ "repo": repo_prop(), "id": id_prop() }), &["repo", "id"]),
        ),
        tool(
            "delete_issue",
            "Delete local issue",
            Destructive,
            false,
            "Deletes a local issue permanently. GitHub issues can't be deleted; close them with update_issue.",
            object(json!({ "repo": repo_prop(), "id": id_prop() }), &["repo", "id"]),
        ),
    ]
}

fn source(args: &Value, default: Source) -> Result<Source> {
    match opt_str(args, "source")? {
        None => Ok(default),
        Some("local") => Ok(Source::Local),
        Some("github") => Ok(Source::Github),
        Some(other) => Err(AppError::InvalidInput(format!("source: {other}"))),
    }
}

fn id(args: &Value) -> Result<u64> {
    let id = opt_u64(args, "id")?.ok_or_else(|| AppError::InvalidInput("id is required".into()))?;
    security::issue_id(id)
}

fn summary(i: &Issue) -> Value {
    json!({
        "source": i.source,
        "id": i.id,
        "title": i.title,
        "state": i.state,
        "labels": i.labels.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
        "author": i.author,
        "updated": iso8601(i.updated_at),
        "comments": i.comments,
        "url": i.url,
        // A publish that stopped halfway: publish_issue again to finish it.
        "publishedAs": i.published_as,
    })
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    let repo = repo(ctx, args)?;
    let repo = repo.as_path();
    match name {
        "list_issues" => {
            let state = match opt_str(args, "state")?.unwrap_or("open") {
                "open" => StateFilter::Open,
                "closed" => StateFilter::Closed,
                "all" => StateFilter::All,
                other => return Err(AppError::InvalidInput(format!("state: {other}"))),
            };
            let which = match opt_str(args, "source")?.unwrap_or("all") {
                s @ ("local" | "github" | "all") => s,
                other => return Err(AppError::InvalidInput(format!("source: {other}"))),
            };
            let slug = issues::github::repo_slug(repo)?;
            let local = match which {
                "local" | "all" => issues::local::list(repo, state)?,
                _ => Vec::new(),
            };
            // With "all", a GitHub failure (offline, rate limit) still returns the local issues.
            let (github, github_error) = match (which, &slug) {
                ("github", Some(_)) => (issues::github::list(repo, state)?, None),
                ("github", None) => return Err(AppError::NoGithubRemote),
                ("all", Some(_)) => match issues::github::list(repo, state) {
                    Ok(list) => (list, None),
                    Err(e) => (Vec::new(), Some(e.to_string())),
                },
                _ => (Vec::new(), None),
            };
            Ok(Output::Json(json!({
                "githubRepo": slug,
                "local": local.iter().map(summary).collect::<Vec<_>>(),
                "github": github.iter().map(summary).collect::<Vec<_>>(),
                "githubError": github_error,
            })))
        }
        "get_issue" => {
            let id = id(args)?;
            let d = match source(args, Source::Github)? {
                Source::Local => issues::local::get(repo, id)?,
                Source::Github => issues::github::get(repo, id)?,
            };
            let mut v = summary(&d.issue);
            v["body"] = json!(d.issue.body);
            v["comments"] = json!(
                d.comments
                    .iter()
                    .map(|c| json!({ "author": c.author, "created": iso8601(c.created_at), "body": c.body }))
                    .collect::<Vec<_>>()
            );
            Ok(Output::Json(v))
        }
        "create_issue" => {
            let title = str_arg(args, "title")?;
            let body = opt_str(args, "body")?.unwrap_or("");
            let issue = match source(args, Source::Local)? {
                Source::Local => issues::local::create(repo, title, body)?,
                Source::Github => issues::github::create(repo, title, body)?,
            };
            Ok(Output::Json(json!({ "message": "created", "issue": summary(&issue) })))
        }
        "update_issue" => {
            let id = id(args)?;
            let state = match opt_str(args, "state")? {
                None => None,
                Some("open") => Some(IssueState::Open),
                Some("closed") => Some(IssueState::Closed),
                Some(other) => return Err(AppError::InvalidInput(format!("state: {other}"))),
            };
            let change = IssueUpdate {
                title: opt_str(args, "title")?.map(String::from),
                body: opt_str(args, "body")?.map(String::from),
                state,
            };
            let issue = match source(args, Source::Github)? {
                Source::Local => issues::local::update(repo, id, &change)?,
                Source::Github => issues::github::update(repo, id, &change)?,
            };
            Ok(Output::Json(json!({ "message": "updated", "issue": summary(&issue) })))
        }
        "comment_issue" => {
            let id = id(args)?;
            let body = str_arg(args, "body")?;
            match source(args, Source::Github)? {
                Source::Local => issues::local::comment(repo, id, body)?,
                Source::Github => issues::github::comment(repo, id, body)?,
            };
            Ok(Output::Json(json!({ "message": "commented" })))
        }
        "publish_issue" => {
            let issue = issues::publish(repo, id(args)?)?;
            Ok(Output::Json(
                json!({ "message": "published", "issue": summary(&issue) }),
            ))
        }
        "delete_issue" => {
            issues::local::delete(repo, id(args)?)?;
            Ok(Output::Json(json!({ "message": "deleted" })))
        }
        _ => Err(no_handler(name)),
    }
}
