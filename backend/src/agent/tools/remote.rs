//! Remote tools: listing the remotes, and fetch, pull and push signed in with the
//! user's GitHub account. Agents cannot add or change remotes (that would redirect pushes).

use std::path::Path;

use serde_json::{Value, json};

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
            "list_remotes",
            "List remotes",
            Read,
            false,
            "The configured remotes with their fetch and push URLs (credentials in a URL are hidden).",
            repo_only(),
        ),
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
        tool(
            "force_push",
            "Force push",
            Destructive,
            true,
            "Pushes the current branch over its upstream even if that removes commits on the remote (--force-with-lease: refused when the remote branch has commits you have not fetched). Use only when the user asked for it, e.g. after rewriting pushed commits.",
            repo_only(),
        ),
    ]
}

fn auth_env(repo: &Path) -> Vec<(String, String)> {
    github::git_auth_env_for_repo(repo)
}

/// `https://user:token@host/x` -> `https://host/x`.
fn hide_credentials(url: &str) -> String {
    match url.split_once("://") {
        Some((scheme, rest)) => {
            let host_end = rest.find('/').unwrap_or(rest.len());
            match rest[..host_end].rfind('@') {
                Some(at) => format!("{scheme}://{}", &rest[at + 1..]),
                None => url.to_string(),
            }
        }
        None => url.to_string(),
    }
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "list_remotes" => {
            let repo = repo(ctx, args)?;
            let remotes: Vec<Value> = git::remotes::list(&repo)?
                .iter()
                .map(|r| {
                    json!({
                        "name": r.name,
                        "fetchUrl": hide_credentials(&r.fetch_url),
                        "pushUrl": r.push_url.as_deref().map(hide_credentials),
                    })
                })
                .collect();
            Ok(Output::Json(json!(remotes)))
        }
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
        "force_push" => {
            let repo = repo(ctx, args)?;
            git::remote::force_push(&repo, auth_env(&repo), |_| {})?;
            outcome(&repo, Outcome::Done)
        }
        "push" => {
            let repo = repo(ctx, args)?;
            git::remote::push(&repo, auth_env(&repo), |_| {})?;
            outcome(&repo, Outcome::Done)
        }
        _ => Err(no_handler(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::hide_credentials;

    #[test]
    fn hides_credentials_in_urls() {
        assert_eq!(
            hide_credentials("https://user:tok@github.com/a/b.git"),
            "https://github.com/a/b.git"
        );
        assert_eq!(
            hide_credentials("https://github.com/a/b.git"),
            "https://github.com/a/b.git"
        );
        assert_eq!(hide_credentials("git@github.com:a/b.git"), "git@github.com:a/b.git");
        assert_eq!(
            hide_credentials("https://github.com/a@b/c.git"),
            "https://github.com/a@b/c.git"
        );
    }
}
