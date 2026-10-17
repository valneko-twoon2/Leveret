//! Reading tool arguments and resolving what they name: registered repositories,
//! commits and branches. Every value is validated like the app's commands do.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::Context;
use crate::error::{AppError, Result};
use crate::git::branch::BranchKind;
use crate::git::{self, Git, decode};
use crate::repos::RepoStore;
use crate::security;

/// Default and maximum size of diff text returned to an agent.
const DEFAULT_MAX_BYTES: usize = 100_000;
pub(super) const MAX_BYTES: usize = 1_000_000;

pub(super) fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    opt_str(args, key)?.ok_or_else(|| AppError::InvalidInput(format!("missing argument '{key}'")))
}

pub(super) fn opt_str<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.as_str())),
        Some(_) => Err(AppError::InvalidInput(format!("argument '{key}' must be a string"))),
    }
}

pub(super) fn opt_bool(args: &Value, key: &str) -> Result<Option<bool>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(AppError::InvalidInput(format!(
            "argument '{key}' must be true or false"
        ))),
    }
}

pub(super) fn opt_u64(args: &Value, key: &str) -> Result<Option<u64>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .ok_or_else(|| AppError::InvalidInput(format!("argument '{key}' must be a non-negative integer"))),
    }
}

pub(super) fn path_list(args: &Value, key: &str) -> Result<Option<Vec<String>>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => {
            let mut out = Vec::new();
            for item in items {
                let p = item
                    .as_str()
                    .ok_or_else(|| AppError::InvalidInput(format!("'{key}' must be a list of strings")))?;
                // Agents often use backslashes on Windows; git paths use slashes.
                let p = p.replace('\\', "/");
                security::relative_path(&p)?;
                out.push(p);
            }
            Ok(Some(out))
        }
        Some(_) => Err(AppError::InvalidInput(format!("'{key}' must be a list of strings"))),
    }
}

pub(super) fn max_bytes(args: &Value) -> Result<usize> {
    Ok(opt_u64(args, "maxBytes")?.map_or(DEFAULT_MAX_BYTES, |m| (m as usize).clamp(1000, MAX_BYTES)))
}

/// Finds a registered repository by path, name, or a path inside it.
fn resolve_repo(store: &RepoStore, input: &str) -> Result<PathBuf> {
    let norm = |s: &str| s.trim().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let want = norm(input);
    let list = store.list();
    if let Some(r) = list.repos.iter().find(|r| norm(&r.path) == want) {
        return Ok(PathBuf::from(&r.path));
    }
    let by_name: Vec<_> = list.repos.iter().filter(|r| r.name.to_lowercase() == want).collect();
    match by_name.len() {
        1 => return Ok(PathBuf::from(&by_name[0].path)),
        n if n > 1 => {
            return Err(AppError::InvalidInput(format!(
                "several repositories are named '{input}'; pass the path"
            )));
        }
        _ => {}
    }
    let inside = list
        .repos
        .iter()
        .filter(|r| want.starts_with(&format!("{}\\", norm(&r.path))))
        .max_by_key(|r| r.path.len());
    if let Some(r) = inside {
        return Ok(PathBuf::from(&r.path));
    }
    let names: Vec<&str> = list.repos.iter().map(|r| r.name.as_str()).collect();
    Err(AppError::RepoNotRegistered(format!(
        "{input} (registered: {})",
        names.join(", ")
    )))
}

pub(super) fn repo(ctx: &Context, args: &Value) -> Result<PathBuf> {
    resolve_repo(ctx.store, str_arg(args, "repo")?)
}

/// Resolves a revision (hash, HEAD~1, branch, ...) to a full commit hash.
pub(super) fn resolve_commit(repo: &Path, rev: &str) -> Result<String> {
    security::revision_expr(rev)?;
    let out = Git::new(repo)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{rev}^{{commit}}"))
        .ok_codes(&[0, 1])
        .run()?;
    let hash = decode(&out.stdout).trim().to_string();
    if out.code != 0 || security::object_id(&hash).is_err() {
        return Err(AppError::InvalidInput(format!("unknown commit '{rev}'")));
    }
    Ok(hash)
}

/// Resolves `main`, `origin/main` or a full ref to the full ref of an existing branch.
pub(super) fn resolve_branch(repo: &Path, name: &str) -> Result<String> {
    let branches = git::branch::list(repo)?;
    let found = branches
        .iter()
        .find(|b| b.full_ref == name)
        .or_else(|| branches.iter().find(|b| b.kind == BranchKind::Local && b.name == name))
        .or_else(|| branches.iter().find(|b| b.kind == BranchKind::Remote && b.name == name));
    match found {
        Some(b) => Ok(security::branch_ref(&b.full_ref)?.to_string()),
        None => Err(AppError::InvalidInput(format!(
            "branch '{name}' not found; call list_branches"
        ))),
    }
}
