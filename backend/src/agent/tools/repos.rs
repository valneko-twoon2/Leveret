//! Repository tools: listing, registering and cloning repositories, and their status.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::args::{opt_str, repo, str_arg};
use super::schema::{object, repo_only, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::{AppError, Result};
use crate::{git, github, security};

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "list_repositories",
            "List repositories",
            Read,
            false,
            "Lists the repositories registered in Leveret (name and path). Other tools take one of these as `repo`.",
            object(json!({}), &[]),
        ),
        tool(
            "get_status",
            "Get status",
            Read,
            false,
            "Current branch, upstream with ahead/behind counts, a merge/rebase/revert in progress, the number of stashes, and every changed file with its status (modified, added, deleted, renamed, untracked, conflicted, ...).",
            repo_only(),
        ),
        tool(
            "add_repository",
            "Add repository",
            Write,
            false,
            "Registers an existing local git repository in Leveret so the other tools can use it.",
            object(
                json!({ "path": string("Absolute path of the repository (or a folder inside it).") }),
                &["path"],
            ),
        ),
        tool(
            "clone_repository",
            "Clone repository",
            Write,
            true,
            "Clones a remote repository (https or ssh URL) and registers it. Uses the user's GitHub sign-in for github.com.",
            object(
                json!({
                    "url": string("Repository URL, e.g. https://github.com/owner/name.git"),
                    "parentDir": string("Absolute folder to clone into (default: Documents\\GitHub)."),
                    "name": string("Folder name (default: from the URL)."),
                }),
                &["url"],
            ),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "list_repositories" => {
            let list = ctx.store.list();
            Ok(Output::Json(json!(
                list.repos
                    .iter()
                    .map(|r| json!({ "name": r.name, "path": r.path }))
                    .collect::<Vec<_>>()
            )))
        }
        "get_status" => {
            let repo = repo(ctx, args)?;
            let st = git::status::get(&repo)?;
            let stashes = git::stash::list(&repo)?.len();
            let mut v = serde_json::to_value(&st).map_err(|e| AppError::Other(e.to_string()))?;
            v["repository"] = json!(repo.display().to_string());
            v["stashCount"] = json!(stashes);
            Ok(Output::Json(v))
        }
        "add_repository" => {
            let path = PathBuf::from(str_arg(args, "path")?);
            if !path.is_absolute() {
                return Err(AppError::InvalidInput("path must be absolute".into()));
            }
            let entry = ctx.store.add(&path, false)?;
            Ok(Output::Json(
                json!({ "message": "registered", "name": entry.name, "path": entry.path }),
            ))
        }
        "clone_repository" => {
            let url = str_arg(args, "url")?;
            security::clone_url(url)?;
            let name = match opt_str(args, "name")? {
                Some(n) => n.to_string(),
                None => repo_name_from_url(url),
            };
            security::folder_name(&name)?;
            let parent = match opt_str(args, "parentDir")? {
                Some(p) => PathBuf::from(p),
                None => dirs::document_dir()
                    .ok_or_else(|| AppError::Other("no Documents folder".into()))?
                    .join("GitHub"),
            };
            if !parent.is_absolute() {
                return Err(AppError::InvalidInput("parentDir must be absolute".into()));
            }
            let dest = parent.join(&name);
            git::clone::clone(url, &dest, github::git_auth_env(url), |_| {})?;
            let entry = ctx.store.add(&dest, false)?;
            Ok(Output::Json(
                json!({ "message": "cloned", "name": entry.name, "path": entry.path }),
            ))
        }
        _ => Err(no_handler(name)),
    }
}

/// Folder name git would create for a clone URL (`.../name.git` -> `name`).
fn repo_name_from_url(url: &str) -> String {
    let last = url
        .trim()
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\', ':'])
        .next()
        .unwrap_or_default();
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(repo_name_from_url("https://github.com/a/leveret.git"), "leveret");
        assert_eq!(repo_name_from_url("git@github.com:a/b"), "b");
    }
}
