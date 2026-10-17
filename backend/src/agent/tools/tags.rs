//! Tag tools: listing, creating, pushing and deleting tags.

use serde_json::{Value, json};

use super::args::{opt_bool, opt_str, repo, resolve_commit, str_arg};
use super::schema::{boolean, object, repo_only, repo_prop, string, tool};
use super::{Context, Output, Tool, no_handler};
use crate::agent::Level;
use crate::error::Result;
use crate::{git, github, security};

pub(super) fn tools() -> Vec<Tool> {
    use Level::*;
    vec![
        tool(
            "list_tags",
            "List tags",
            Read,
            false,
            "Lists tags, newest first: name, commit, whether annotated, and the message (or commit) subject.",
            repo_only(),
        ),
        tool(
            "create_tag",
            "Create tag",
            Write,
            false,
            "Creates a tag at a commit (HEAD by default). With `message` the tag is annotated, otherwise lightweight. Tags are local until push_tag.",
            object(
                json!({
                    "repo": repo_prop(),
                    "name": string("Tag name, e.g. v1.2.0."),
                    "commit": string("Commit to tag (hash, HEAD~1, branch, ...). Default: HEAD."),
                    "message": string("Tag message; makes an annotated tag."),
                }),
                &["repo", "name"],
            ),
        ),
        tool(
            "push_tag",
            "Push tag",
            Write,
            true,
            "Pushes a tag to the default remote (origin, or the only remote).",
            object(
                json!({ "repo": repo_prop(), "name": string("Tag name.") }),
                &["repo", "name"],
            ),
        ),
        tool(
            "delete_tag",
            "Delete tag",
            Destructive,
            true,
            "Deletes a local tag. With `remote`, also deletes it from the default remote, which affects everyone using that remote.",
            object(
                json!({
                    "repo": repo_prop(),
                    "name": string("Tag name."),
                    "remote": boolean("Also delete the tag on the remote."),
                }),
                &["repo", "name"],
            ),
        ),
    ]
}

pub(super) fn run(ctx: &Context, name: &str, args: &Value) -> Result<Output> {
    match name {
        "list_tags" => {
            let repo = repo(ctx, args)?;
            let tags: Vec<Value> = git::tag::list(&repo)?
                .into_iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "commit": t.target.chars().take(12).collect::<String>(),
                        "annotated": t.annotated,
                        "subject": t.subject,
                    })
                })
                .collect();
            Ok(Output::Json(json!({ "tags": tags })))
        }
        "create_tag" => {
            let repo = repo(ctx, args)?;
            let tag = security::tag_name(str_arg(args, "name")?)?;
            let commit = resolve_commit(&repo, opt_str(args, "commit")?.unwrap_or("HEAD"))?;
            let message = security::tag_message(opt_str(args, "message")?.unwrap_or_default())?;
            git::tag::create(&repo, tag, &commit, message)?;
            Ok(Output::Json(
                json!({ "message": "created", "tag": tag, "commit": commit, "next": "push_tag to publish it" }),
            ))
        }
        "push_tag" => {
            let repo = repo(ctx, args)?;
            let tag = security::tag_name(str_arg(args, "name")?)?;
            git::tag::push(&repo, github::git_auth_env_for_repo(&repo), tag)?;
            Ok(Output::Json(json!({ "message": "pushed", "tag": tag })))
        }
        "delete_tag" => {
            let repo = repo(ctx, args)?;
            let tag = security::tag_name(str_arg(args, "name")?)?;
            let remote = opt_bool(args, "remote")?.unwrap_or(false);
            if remote {
                git::tag::delete_remote(&repo, github::git_auth_env_for_repo(&repo), tag)?;
            }
            git::tag::delete(&repo, tag)?;
            Ok(Output::Json(
                json!({ "message": "deleted", "tag": tag, "remote": remote }),
            ))
        }
        _ => Err(no_handler(name)),
    }
}
