//! Branch tools: listing, creating, switching and deleting branches, and checking
//! out a commit.

use serde_json::{Value, json};

use super::args::{opt_bool, opt_str, repo, resolve_branch, resolve_commit, str_arg};
use super::render::outcome;
use super::schema::{boolean, object, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::Result;
use crate::git::branch::BranchKind;
use crate::git::operation::Outcome;
use crate::{git, security};

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "list_branches",
            "List branches",
            Read,
            false,
            "Local and remote branches with the current one marked, upstreams, ahead/behind counts and the latest commit.",
            object(
                json!({ "repo": repo_prop(), "includeRemote": boolean("Include remote branches (default true).") }),
                &["repo"],
            ),
        ),
        tool(
            "create_branch",
            "Create branch",
            Write,
            false,
            "Creates a branch at `startPoint` (default: HEAD) and switches to it unless `checkout` is false. Uncommitted changes are carried over.",
            object(
                json!({
                    "repo": repo_prop(),
                    "name": string("New branch name, e.g. feature/login."),
                    "startPoint": string("Commit hash, revision or branch to start from (default HEAD)."),
                    "checkout": boolean("Switch to the new branch (default true)."),
                }),
                &["repo", "name"],
            ),
        ),
        tool(
            "switch_branch",
            "Switch branch",
            Write,
            false,
            "Switches to a branch. A remote branch such as origin/feature is checked out as a local tracking branch.",
            object(
                json!({ "repo": repo_prop(), "branch": string("Branch name, e.g. main or origin/feature.") }),
                &["repo", "branch"],
            ),
        ),
        tool(
            "checkout_commit",
            "Check out commit",
            Write,
            false,
            "Checks out a commit without a branch (detached HEAD).",
            object(
                json!({ "repo": repo_prop(), "commit": string("Commit hash or revision.") }),
                &["repo", "commit"],
            ),
        ),
        tool(
            "delete_branch",
            "Delete branch",
            Destructive,
            false,
            "Deletes a local branch. Without `force`, branches with unmerged commits are refused.",
            object(
                json!({ "repo": repo_prop(), "name": string("Local branch name."), "force": boolean("Delete even if not merged.") }),
                &["repo", "name"],
            ),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "list_branches" => {
            let repo = repo(ctx, args)?;
            let remote = opt_bool(args, "includeRemote")?.unwrap_or(true);
            let branches: Vec<_> = git::branch::list(&repo)?
                .into_iter()
                .filter(|b| remote || b.kind == BranchKind::Local)
                .collect();
            Ok(Output::Json(json!(branches)))
        }
        "create_branch" => {
            let repo = repo(ctx, args)?;
            let name = str_arg(args, "name")?;
            security::branch_name(name)?;
            let start = match opt_str(args, "startPoint")? {
                Some(s) => Some(match resolve_branch(&repo, s) {
                    Ok(full) => full,
                    Err(_) => resolve_commit(&repo, s)?,
                }),
                None => None,
            };
            let checkout = opt_bool(args, "checkout")?.unwrap_or(true);
            git::branch::create(&repo, name, start.as_deref(), checkout)?;
            outcome(&repo, Outcome::Done)
        }
        "switch_branch" => {
            let repo = repo(ctx, args)?;
            let full = resolve_branch(&repo, str_arg(args, "branch")?)?;
            git::branch::switch(&repo, &full)?;
            outcome(&repo, Outcome::Done)
        }
        "checkout_commit" => {
            let repo = repo(ctx, args)?;
            let hash = resolve_commit(&repo, str_arg(args, "commit")?)?;
            git::branch::checkout_detached(&repo, &hash)?;
            outcome(&repo, Outcome::Done)
        }
        "delete_branch" => {
            let repo = repo(ctx, args)?;
            let name = str_arg(args, "name")?;
            security::branch_name(name)?;
            git::branch::delete(&repo, name, opt_bool(args, "force")?.unwrap_or(false))?;
            outcome(&repo, Outcome::Done)
        }
        _ => Err(no_handler(name)),
    }
}
