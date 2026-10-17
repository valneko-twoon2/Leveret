//! Operation tools: merge, rebase, revert and cherry-pick, and resolving,
//! continuing, skipping or aborting one that stopped on conflicts.

use serde_json::{Value, json};

use super::args::{path_list, repo, resolve_branch, resolve_commit, str_arg};
use super::render::outcome;
use super::schema::{object, paths, repo_only, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::Result;
use crate::git;
use crate::git::operation::Outcome;

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "merge_branch",
            "Merge branch",
            Write,
            false,
            "Merges a branch into the current branch. May stop with conflicts.",
            object(
                json!({ "repo": repo_prop(), "branch": string("Branch to merge, e.g. main or origin/main.") }),
                &["repo", "branch"],
            ),
        ),
        tool(
            "rebase_branch",
            "Rebase",
            Write,
            false,
            "Rebases the current branch onto another branch. Uncommitted changes are stashed and restored automatically. May stop with conflicts.",
            object(
                json!({ "repo": repo_prop(), "onto": string("Branch to rebase onto, e.g. main or origin/main.") }),
                &["repo", "onto"],
            ),
        ),
        tool(
            "revert_commit",
            "Revert commit",
            Write,
            false,
            "Creates a new commit that undoes the changes of a commit. May stop with conflicts.",
            object(
                json!({ "repo": repo_prop(), "commit": string("Commit hash or revision.") }),
                &["repo", "commit"],
            ),
        ),
        tool(
            "cherry_pick",
            "Cherry-pick",
            Write,
            false,
            "Applies the changes of a commit to the current branch as a new commit. May stop with conflicts.",
            object(
                json!({ "repo": repo_prop(), "commit": string("Commit hash or revision.") }),
                &["repo", "commit"],
            ),
        ),
        tool(
            "mark_resolved",
            "Mark resolved",
            Write,
            false,
            "Marks conflicted files as resolved after their conflict markers have been fixed.",
            object(
                json!({ "repo": repo_prop(), "paths": paths("Files to mark as resolved.") }),
                &["repo", "paths"],
            ),
        ),
        tool(
            "continue_operation",
            "Continue",
            Write,
            false,
            "Continues the merge, rebase, revert or cherry-pick in progress once every conflict is resolved.",
            repo_only(),
        ),
        tool(
            "abort_operation",
            "Abort",
            Destructive,
            false,
            "Aborts the merge, rebase, revert or cherry-pick in progress and returns to the state before it (conflict resolutions are lost).",
            repo_only(),
        ),
        tool(
            "skip_operation",
            "Skip commit",
            Destructive,
            false,
            "Skips the current commit of a rebase, revert or cherry-pick (its changes are dropped).",
            repo_only(),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "merge_branch" => {
            let repo = repo(ctx, args)?;
            let full = resolve_branch(&repo, str_arg(args, "branch")?)?;
            let o = git::operation::merge(&repo, &full)?;
            outcome(&repo, o)
        }
        "rebase_branch" => {
            let repo = repo(ctx, args)?;
            let full = resolve_branch(&repo, str_arg(args, "onto")?)?;
            let o = git::operation::rebase(&repo, &full)?;
            outcome(&repo, o)
        }
        "revert_commit" | "cherry_pick" => {
            let repo = repo(ctx, args)?;
            let hash = resolve_commit(&repo, str_arg(args, "commit")?)?;
            let is_merge = git::log::detail(&repo, &hash)?.parents.len() > 1;
            let o = if name == "revert_commit" {
                git::operation::revert(&repo, &hash, is_merge)?
            } else {
                git::operation::cherry_pick(&repo, &hash, is_merge)?
            };
            outcome(&repo, o)
        }
        "mark_resolved" => {
            let repo = repo(ctx, args)?;
            let paths = path_list(args, "paths")?.unwrap_or_default();
            git::operation::mark_resolved(&repo, &paths)?;
            let remaining = git::operation::conflicted_paths(&repo)?;
            Ok(Output::Json(
                json!({ "message": "resolved", "remainingConflicts": remaining }),
            ))
        }
        "continue_operation" => {
            let repo = repo(ctx, args)?;
            let o = git::operation::continue_(&repo)?;
            outcome(&repo, o)
        }
        "abort_operation" => {
            let repo = repo(ctx, args)?;
            git::operation::abort(&repo)?;
            outcome(&repo, Outcome::Done)
        }
        "skip_operation" => {
            let repo = repo(ctx, args)?;
            let o = git::operation::skip(&repo)?;
            outcome(&repo, o)
        }
        _ => Err(no_handler(name)),
    }
}
