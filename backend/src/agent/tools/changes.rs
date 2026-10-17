//! Working tree tools: the uncommitted diff, committing, undoing the last commit,
//! ignoring files and discarding changes.

use serde_json::{Value, json};

use super::args::{MAX_BYTES, max_bytes, opt_bool, opt_str, path_list, repo, str_arg};
use super::render::{brief_status, outcome, working_diff};
use super::schema::{boolean, integer, object, paths, repo_only, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::Result;
use crate::git::operation::Outcome;
use crate::{git, security};

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "get_diff",
            "Get diff",
            Read,
            false,
            "Unified diff of uncommitted changes against HEAD, including the content of untracked files. Give `path` for a single file. Long output is cut at `maxBytes`.",
            object(
                json!({
                    "repo": repo_prop(),
                    "path": string("Only this file (relative to the repository root)."),
                    "maxBytes": integer("Maximum size of the returned text (default 100000).", 1000, MAX_BYTES as u64),
                }),
                &["repo"],
            ),
        ),
        tool(
            "commit",
            "Commit",
            Write,
            false,
            "Commits changes. Without `paths`, every changed file is included. `amend` replaces the last (unpushed) commit instead.",
            object(
                json!({
                    "repo": repo_prop(),
                    "summary": string("First line of the commit message."),
                    "description": string("Rest of the commit message."),
                    "paths": paths("Files to commit (relative paths as in get_status). Default: all changed files."),
                    "amend": boolean("Replace the last commit (only if it has not been pushed)."),
                }),
                &["repo", "summary"],
            ),
        ),
        tool(
            "undo_last_commit",
            "Undo last commit",
            Write,
            false,
            "Removes the last commit if it has not been pushed, keeping its changes in the working tree. Returns its message.",
            repo_only(),
        ),
        tool(
            "ignore_path",
            "Ignore file",
            Write,
            false,
            "Adds a file, or every file with its extension, to .gitignore.",
            object(
                json!({
                    "repo": repo_prop(),
                    "path": string("File relative to the repository root."),
                    "byExtension": boolean("Ignore all files with this file's extension instead."),
                }),
                &["repo", "path"],
            ),
        ),
        tool(
            "discard_changes",
            "Discard changes",
            Destructive,
            false,
            "Discards uncommitted changes of files (they are moved to the Recycle Bin, then restored from HEAD; untracked files are removed).",
            object(
                json!({ "repo": repo_prop(), "paths": paths("Files to discard (relative paths as in get_status).") }),
                &["repo", "paths"],
            ),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "get_diff" => {
            let repo = repo(ctx, args)?;
            let max = max_bytes(args)?;
            let only = opt_str(args, "path")?.map(|p| p.replace('\\', "/"));
            if let Some(p) = &only {
                security::relative_path(p)?;
            }
            Ok(Output::Text(working_diff(&repo, only.as_deref(), max)?))
        }
        "commit" => {
            let repo = repo(ctx, args)?;
            let amend = opt_bool(args, "amend")?.unwrap_or(false);
            let paths = match path_list(args, "paths")? {
                Some(p) => p,
                None => git::status::get(&repo)?
                    .files
                    .iter()
                    .flat_map(|f| std::iter::once(f.path.clone()).chain(f.orig_path.clone()))
                    .collect(),
            };
            let req = git::commit::CommitRequest {
                summary: str_arg(args, "summary")?.to_string(),
                description: opt_str(args, "description")?.unwrap_or_default().to_string(),
                paths,
                amend,
            };
            let hash = git::commit::commit(&repo, &req)?;
            Ok(Output::Json(
                json!({ "message": if amend { "amended" } else { "committed" }, "commit": hash, "status": brief_status(&repo)? }),
            ))
        }
        "undo_last_commit" => {
            let repo = repo(ctx, args)?;
            let message = git::commit::undo_last(&repo)?;
            Ok(Output::Json(
                json!({ "message": "undone", "commitMessage": message, "note": "The changes are kept in the working tree." }),
            ))
        }
        "ignore_path" => {
            let repo = repo(ctx, args)?;
            let path = str_arg(args, "path")?.replace('\\', "/");
            security::relative_path(&path)?;
            git::commit::ignore(&repo, &path, opt_bool(args, "byExtension")?.unwrap_or(false))?;
            outcome(&repo, Outcome::Done)
        }
        "discard_changes" => {
            let repo = repo(ctx, args)?;
            let paths = path_list(args, "paths")?.unwrap_or_default();
            git::discard::discard(&repo, &paths)?;
            outcome(&repo, Outcome::Done)
        }
        _ => Err(no_handler(name)),
    }
}
