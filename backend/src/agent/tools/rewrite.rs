//! Rewriting tools: interactive rebase (reorder, squash, reword, drop), resetting
//! the current branch to a commit, and restoring a file to a commit's version
//! (or undoing what a commit did to it).

use serde_json::{Value, json};

use super::args::{opt_str, repo, resolve_commit, str_arg};
use super::render::{iso8601, outcome};
use super::schema::{object, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::{AppError, Result};
use crate::git::operation::Outcome;
use crate::git::rebase::{Action, Step};
use crate::git::reset::ResetMode;
use crate::{git, security};

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "get_rebase_plan",
            "Get commits to rebase",
            Read,
            false,
            "The commits after `base` up to HEAD, oldest first, with full messages and whether each was pushed: the input for interactive_rebase. Without `base`, from the first commit. Fails if the range contains merge commits.",
            object(
                json!({ "repo": repo_prop(), "base": string("The commit before the first one to rewrite (hash, HEAD~3, ...).") }),
                &["repo"],
            ),
        ),
        tool(
            "interactive_rebase",
            "Interactive rebase",
            Destructive,
            false,
            "Rewrites the commits after `base` (see get_rebase_plan): `steps` lists every one of them exactly once, oldest first, in the new order, each with an action: pick, reword (with `message`), squash (meld into the previous kept commit, keeping both messages), fixup (meld, dropping this message) or drop. Uncommitted changes are stashed and restored. Pushed commits are refused. May stop on conflicts (resolve, then continue_operation or abort_operation).",
            object(
                json!({
                    "repo": repo_prop(),
                    "base": string("The commit before the first one to rewrite. Omit to start from the first commit."),
                    "steps": {
                        "type": "array",
                        "description": "Every commit of the range once, oldest first, in the new order.",
                        "items": object(
                            json!({
                                "commit": string("Full commit hash from get_rebase_plan."),
                                "action": { "type": "string", "enum": ["pick", "reword", "squash", "fixup", "drop"] },
                                "message": string("The new message for reword."),
                            }),
                            &["commit", "action"],
                        ),
                    },
                }),
                &["repo", "steps"],
            ),
        ),
        tool(
            "reset_branch",
            "Reset branch to commit",
            Destructive,
            false,
            "Moves the current branch to another commit (git reset). `keep` (default) leaves the changes of the commits that leave the branch as uncommitted changes; `discard` throws them away (refused while there are uncommitted changes). Commits that were pushed then need a force push, so prefer this for unpushed work.",
            object(
                json!({
                    "repo": repo_prop(),
                    "commit": string("Commit to move the branch to (hash, HEAD~2, a tag, ...)."),
                    "mode": { "type": "string", "enum": ["keep", "discard"], "description": "What happens to the changes of the removed commits (default keep)." },
                }),
                &["repo", "commit"],
            ),
        ),
        tool(
            "restore_file",
            "Restore file from commit",
            Destructive,
            false,
            "Changes one file in the working tree: `version` (default) puts back its content from a commit; `undo` reverses what that commit did to the file, keeping later changes (fails if they overlap). The current file is copied to the Recycle Bin first.",
            object(
                json!({
                    "repo": repo_prop(),
                    "path": string("File relative to the repository root."),
                    "commit": string("The commit (hash, HEAD~1, branch, ...)."),
                    "mode": { "type": "string", "enum": ["version", "undo"], "description": "Put back the commit's version, or undo the commit's change (default version)." },
                }),
                &["repo", "path", "commit"],
            ),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "get_rebase_plan" => {
            let repo = repo(ctx, args)?;
            let base = opt_str(args, "base")?.map(|b| resolve_commit(&repo, b)).transpose()?;
            let plan = git::rebase::plan(&repo, base.as_deref())?;
            let out: Vec<Value> = plan
                .iter()
                .map(|c| {
                    json!({
                        "commit": c.hash,
                        "message": c.message,
                        "author": c.author_name,
                        "date": iso8601(c.author_time),
                        "pushed": c.pushed,
                    })
                })
                .collect();
            Ok(Output::Json(json!({ "base": base, "commits": out })))
        }
        "interactive_rebase" => {
            let repo = repo(ctx, args)?;
            let base = opt_str(args, "base")?.map(|b| resolve_commit(&repo, b)).transpose()?;
            let items = args
                .get("steps")
                .and_then(Value::as_array)
                .ok_or_else(|| AppError::InvalidInput("`steps` must be an array".into()))?;
            let mut steps = Vec::new();
            for item in items {
                let hash = str_arg(item, "commit")?;
                security::object_id(hash)?;
                let action = match str_arg(item, "action")? {
                    "pick" => Action::Pick,
                    "reword" => Action::Reword,
                    "squash" => Action::Squash,
                    "fixup" => Action::Fixup,
                    "drop" => Action::Drop,
                    other => return Err(AppError::InvalidInput(format!("unknown action '{other}'"))),
                };
                let message = opt_str(item, "message")?
                    .map(|m| security::commit_message(m).map(String::from))
                    .transpose()?;
                steps.push(Step {
                    hash: hash.to_string(),
                    action,
                    message,
                });
            }
            let o = git::rebase::run(&repo, base.as_deref(), &steps)?;
            outcome(&repo, o)
        }
        "reset_branch" => {
            let repo = repo(ctx, args)?;
            let target = resolve_commit(&repo, str_arg(args, "commit")?)?;
            let mode = match opt_str(args, "mode")?.unwrap_or("keep") {
                "keep" => ResetMode::Keep,
                "discard" => ResetMode::Discard,
                other => return Err(AppError::InvalidInput(format!("unknown mode '{other}'"))),
            };
            git::reset::reset(&repo, &target, mode)?;
            outcome(&repo, Outcome::Done)
        }
        "restore_file" => {
            let repo = repo(ctx, args)?;
            let path = str_arg(args, "path")?.replace('\\', "/");
            security::relative_path(&path)?;
            let rev = resolve_commit(&repo, str_arg(args, "commit")?)?;
            match opt_str(args, "mode")?.unwrap_or("version") {
                "version" => git::reset::restore_file(&repo, &rev, &path)?,
                "undo" => {
                    let file = git::log::detail(&repo, &rev)?
                        .files
                        .into_iter()
                        .find(|f| f.path == path);
                    let orig = file.and_then(|f| f.orig_path);
                    git::reset::undo_file_change(&repo, &rev, &path, orig.as_deref())?
                }
                other => return Err(AppError::InvalidInput(format!("unknown mode '{other}'"))),
            }
            Ok(Output::Json(
                json!({ "message": "restored", "status": super::render::brief_status(&repo)? }),
            ))
        }
        _ => Err(no_handler(name)),
    }
}
