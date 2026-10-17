//! Stash tools: listing, saving, restoring and deleting stashes.

use serde_json::{Value, json};

use super::args::{opt_str, opt_u64, repo};
use super::render::outcome;
use super::schema::{integer, object, repo_only, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::{AppError, Result};
use crate::git;
use crate::git::operation::Outcome;

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "list_stashes",
            "List stashes",
            Read,
            false,
            "Stashed changes, newest first. `index` is what stash_pop and stash_drop take.",
            repo_only(),
        ),
        tool(
            "stash_save",
            "Stash changes",
            Write,
            false,
            "Stashes all uncommitted changes, including untracked files.",
            object(
                json!({ "repo": repo_prop(), "message": string("Optional description.") }),
                &["repo"],
            ),
        ),
        tool(
            "stash_pop",
            "Restore stash",
            Write,
            false,
            "Restores a stash and removes it from the list (kept if it conflicts).",
            object(
                json!({ "repo": repo_prop(), "index": integer("Stash index from list_stashes (default 0).", 0, 10_000) }),
                &["repo"],
            ),
        ),
        tool(
            "stash_drop",
            "Delete stash",
            Destructive,
            false,
            "Deletes a stash permanently.",
            object(
                json!({ "repo": repo_prop(), "index": integer("Stash index from list_stashes.", 0, 10_000) }),
                &["repo", "index"],
            ),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "list_stashes" => {
            let repo = repo(ctx, args)?;
            Ok(Output::Json(json!(git::stash::list(&repo)?)))
        }
        "stash_save" => {
            let repo = repo(ctx, args)?;
            let message = opt_str(args, "message")?.unwrap_or_default();
            if message.len() > 1000 || message.chars().any(char::is_control) {
                return Err(AppError::InvalidInput("stash message".into()));
            }
            git::stash::save(&repo, message, true)?;
            outcome(&repo, Outcome::Done)
        }
        "stash_pop" => {
            let repo = repo(ctx, args)?;
            let index = opt_u64(args, "index")?.unwrap_or(0) as u32;
            let o = git::stash::pop(&repo, index)?;
            outcome(&repo, o)
        }
        "stash_drop" => {
            let repo = repo(ctx, args)?;
            let index = opt_u64(args, "index")?
                .ok_or_else(|| AppError::InvalidInput("missing argument 'index'".into()))?
                as u32;
            git::stash::drop(&repo, index)?;
            outcome(&repo, Outcome::Done)
        }
        _ => Err(no_handler(name)),
    }
}
