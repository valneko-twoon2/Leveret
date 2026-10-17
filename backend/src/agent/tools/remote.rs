//! Remote tools: fetch, pull and push, signed in with the user's GitHub account.

use std::path::Path;

use serde_json::Value;

use super::args::repo;
use super::render::outcome;
use super::schema::{repo_only, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::Result;
use crate::git::operation::Outcome;
use crate::{git, github};

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "fetch",
            "Fetch",
            Write,
            true,
            "Fetches all remotes (and prunes deleted remote branches). Updates the ahead/behind counts.",
            repo_only(),
        ),
        tool(
            "pull",
            "Pull",
            Write,
            true,
            "Pulls the upstream of the current branch. May stop with conflicts.",
            repo_only(),
        ),
        tool(
            "push",
            "Push",
            Write,
            true,
            "Pushes the current branch, or publishes it (sets the upstream) if it has none.",
            repo_only(),
        ),
    ]
}

fn auth_env(repo: &Path) -> Vec<(String, String)> {
    github::git_auth_env_for_repo(repo)
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "fetch" => {
            let repo = repo(ctx, args)?;
            git::remote::fetch(&repo, auth_env(&repo), |_| {})?;
            outcome(&repo, Outcome::Done)
        }
        "pull" => {
            let repo = repo(ctx, args)?;
            let o = git::remote::pull(&repo, auth_env(&repo), |_| {})?;
            outcome(&repo, o)
        }
        "push" => {
            let repo = repo(ctx, args)?;
            git::remote::push(&repo, auth_env(&repo), |_| {})?;
            outcome(&repo, Outcome::Done)
        }
        _ => Err(no_handler(name)),
    }
}
