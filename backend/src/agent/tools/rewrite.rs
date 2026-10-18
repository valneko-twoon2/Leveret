//! Rewriting tools: resetting the current branch to a commit, and restoring a
//! file to a commit's version (or undoing what a commit did to it).

use serde_json::{Value, json};

use super::args::{opt_str, repo, resolve_commit, str_arg};
use super::render::outcome;
use super::schema::{object, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::{AppError, Result};
use crate::git::operation::Outcome;
use crate::git::reset::ResetMode;
use crate::{git, security};

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
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
