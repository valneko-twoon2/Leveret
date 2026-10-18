//! History tools: the commit log and the details of one commit.

use serde_json::{Value, json};

use super::args::{MAX_BYTES, max_bytes, opt_bool, opt_str, opt_u64, repo, resolve_commit, str_arg};
use super::render::{commit_diff, iso8601};
use super::schema::{boolean, integer, object, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::commands::history::check_query;
use crate::error::Result;
use crate::git;

const DEFAULT_LOG_LIMIT: u32 = 30;
const MAX_LOG_LIMIT: u32 = 500;

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "get_log",
            "Get history",
            Read,
            false,
            "Commit history, newest first: hash, subject, author, ISO date, parents and branch/tag labels. Filters search the whole history and combine (all must match); `file` gives one file's history across renames.",
            object(
                json!({
                    "repo": repo_prop(),
                    "limit": integer("Number of commits (default 30).", 1, MAX_LOG_LIMIT as u64),
                    "allBranches": boolean("Include all local and remote branches and tags, not just the current branch."),
                    "message": string("Only commits whose message contains this text (case-insensitive; a commit hash also works)."),
                    "author": string("Only commits whose author name or email contains this text (case-insensitive)."),
                    "contains": string("Only commits that add or remove this exact text in a file (git log -S)."),
                    "path": string("Only commits that touch this file or folder (relative to the repository root)."),
                    "file": string("The history of this file, following renames; each commit lists the file's path at that time."),
                }),
                &["repo"],
            ),
        ),
        tool(
            "get_commit",
            "Get commit",
            Read,
            false,
            "Full message, author, committer, changed files and (optionally) the diff of one commit.",
            object(
                json!({
                    "repo": repo_prop(),
                    "commit": string("Commit hash or revision such as HEAD, HEAD~2 or a branch name."),
                    "includeDiff": boolean("Include the diff (default true)."),
                    "maxBytes": integer("Maximum size of the diff text (default 100000).", 1000, MAX_BYTES as u64),
                }),
                &["repo", "commit"],
            ),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "get_log" => {
            let repo = repo(ctx, args)?;
            let limit = opt_u64(args, "limit")?.map_or(DEFAULT_LOG_LIMIT, |l| (l as u32).clamp(1, MAX_LOG_LIMIT));
            let scope = if opt_bool(args, "allBranches")?.unwrap_or(false) {
                git::log::LogScope::All
            } else {
                git::log::LogScope::Head
            };
            let text = |key| opt_str(args, key).map(|v| v.map(str::to_string));
            let path = |key| opt_str(args, key).map(|v| v.map(|s| s.replace('\\', "/")));
            let query = git::log::LogQuery {
                message: text("message")?,
                author: text("author")?,
                content: text("contains")?,
                path: path("path")?,
                file: path("file")?,
            };
            let entries = if query.is_empty() {
                git::log::get(&repo, scope, limit)?
            } else {
                check_query(&query)?;
                git::log::search(&repo, scope, limit, &query)?
            };
            let out: Vec<Value> = entries
                .iter()
                .map(|c| {
                    let mut v = json!({
                        "hash": c.hash,
                        "subject": c.subject,
                        "author": c.author_name,
                        "date": iso8601(c.author_time),
                        "parents": c.parents,
                        "refs": c.refs.iter().map(|r| json!({ "name": r.name, "kind": r.kind, "current": r.current })).collect::<Vec<_>>(),
                    });
                    if let Some(p) = &c.file_path {
                        v["filePath"] = json!(p);
                    }
                    v
                })
                .collect();
            Ok(Output::Json(json!(out)))
        }
        "get_commit" => {
            let repo = repo(ctx, args)?;
            let hash = resolve_commit(&repo, str_arg(args, "commit")?)?;
            let d = git::log::detail(&repo, &hash)?;
            let mut v = json!({
                "hash": d.hash,
                "parents": d.parents,
                "author": format!("{} <{}>", d.author_name, d.author_email),
                "date": iso8601(d.author_time),
                "committer": format!("{} <{}>", d.committer_name, d.committer_email),
                "commitDate": iso8601(d.commit_time),
                "message": d.message,
                "files": d.files,
            });
            if opt_bool(args, "includeDiff")?.unwrap_or(true) {
                v["diff"] = json!(commit_diff(&repo, &hash, max_bytes(args)?)?);
            }
            Ok(Output::Json(v))
        }
        _ => Err(no_handler(name)),
    }
}
