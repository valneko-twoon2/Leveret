//! Stash tools: listing, looking inside, saving, restoring and deleting stashes.

use serde_json::{Value, json};

use super::args::{MAX_BYTES, max_bytes, opt_str, opt_u64, path_list, repo};
use super::render::{outcome, stash_diff};
use super::schema::{integer, object, paths, repo_only, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::{AppError, Result};
use crate::git::operation::Outcome;
use crate::{git, security};

fn index_prop(description: &str) -> Value {
    integer(description, 0, 10_000)
}

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "list_stashes",
            "List stashes",
            Read,
            false,
            "Stashed changes, newest first. `index` is what get_stash, stash_apply, stash_pop and stash_drop take.",
            repo_only(),
        ),
        tool(
            "get_stash",
            "Show stash",
            Read,
            false,
            "The changes a stash holds, as a unified diff (untracked files stashed with it included). Long output is cut at `maxBytes`.",
            object(
                json!({
                    "repo": repo_prop(),
                    "index": index_prop("Stash index from list_stashes (default 0)."),
                    "maxBytes": integer("Maximum size of the returned text (default 100000).", 1000, MAX_BYTES as u64),
                }),
                &["repo"],
            ),
        ),
        tool(
            "stash_save",
            "Stash changes",
            Write,
            false,
            "Stashes uncommitted changes, including untracked files: all of them, or only `paths`.",
            object(
                json!({
                    "repo": repo_prop(),
                    "message": string("Optional description."),
                    "paths": paths("Only these files (relative paths as in get_status). Default: every change."),
                }),
                &["repo"],
            ),
        ),
        tool(
            "stash_apply",
            "Apply stash",
            Write,
            false,
            "Restores a stash's changes but keeps the stash in the list (stash_pop removes it). May stop on conflicts.",
            object(
                json!({ "repo": repo_prop(), "index": index_prop("Stash index from list_stashes (default 0).") }),
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
                json!({ "repo": repo_prop(), "index": index_prop("Stash index from list_stashes (default 0).") }),
                &["repo"],
            ),
        ),
        tool(
            "stash_drop",
            "Delete stash",
            Destructive,
            false,
            "Deletes a stash permanently; its changes are lost unless they were restored with stash_pop first.",
            object(
                json!({ "repo": repo_prop(), "index": index_prop("Stash index from list_stashes.") }),
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
        "get_stash" => {
            let repo = repo(ctx, args)?;
            let index = opt_u64(args, "index")?.unwrap_or(0) as u32;
            let max = max_bytes(args)?;
            let entry = git::stash::list(&repo)?.into_iter().find(|s| s.index == index);
            let entry = entry.ok_or_else(|| AppError::InvalidInput(format!("there is no stash {index}")))?;
            Ok(Output::Text(stash_diff(&repo, &entry.oid, max)?))
        }
        "stash_save" => {
            let repo = repo(ctx, args)?;
            let message = security::stash_message(opt_str(args, "message")?.unwrap_or_default())?;
            let paths = path_list(args, "paths")?.unwrap_or_default();
            git::stash::save(&repo, message, true, &paths)?;
            outcome(&repo, Outcome::Done)
        }
        "stash_apply" | "stash_pop" => {
            let repo = repo(ctx, args)?;
            let index = opt_u64(args, "index")?.unwrap_or(0) as u32;
            let o = if name == "stash_pop" {
                git::stash::pop(&repo, index, None)?
            } else {
                git::stash::apply(&repo, index, None)?
            };
            outcome(&repo, o)
        }
        "stash_drop" => {
            let repo = repo(ctx, args)?;
            let index = opt_u64(args, "index")?
                .ok_or_else(|| AppError::InvalidInput("missing argument 'index'".into()))?
                as u32;
            git::stash::drop(&repo, index, None)?;
            outcome(&repo, Outcome::Done)
        }
        _ => Err(no_handler(name)),
    }
}
