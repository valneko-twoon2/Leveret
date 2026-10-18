//! Interactive rebase: reordering, squashing, rewording and dropping the commits
//! after a base commit.
//!
//! The todo list is built here from validated commit ids only and handed to
//! `git rebase -i` through `GIT_SEQUENCE_EDITOR`, which git runs as
//! `git cat-file blob <todo> > <todo file>`. New messages are stored as blobs
//! too and applied by `exec` lines that contain nothing but blob ids, so no text
//! from the webview or an agent ever reaches a shell.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::operation::{self, Outcome};
use super::{Git, decode};
use crate::error::{AppError, Result};

/// More commits than this are not offered for reordering.
const MAX_COMMITS: usize = 500;
const FIELD: char = '\x1f';
const RECORD: char = '\x1e';

/// One commit of the range, oldest first (the order git applies them in).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanCommit {
    pub hash: String,
    pub subject: String,
    pub message: String,
    pub author_name: String,
    /// Unix seconds.
    pub author_time: i64,
    /// Reachable from a remote branch: rewriting it would need a force push.
    pub pushed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Pick,
    /// Pick with a new message.
    Reword,
    /// Meld into the previous commit, keeping both messages.
    Squash,
    /// Meld into the previous commit, dropping this message.
    Fixup,
    Drop,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub hash: String,
    pub action: Action,
    /// The new message for `reword`.
    #[serde(default)]
    pub message: Option<String>,
}

fn range(base: Option<&str>) -> String {
    base.map_or_else(|| "HEAD".to_string(), |b| format!("{b}..HEAD"))
}

/// The commits after `base` up to HEAD, oldest first (from the first commit
/// when `base` is None). Ranges with merge commits are refused: an interactive
/// rebase would flatten them.
pub fn plan(repo: &Path, base: Option<&str>) -> Result<Vec<PlanCommit>> {
    if let Some(b) = base {
        let ancestor = Git::new(repo)
            .args(["merge-base", "--is-ancestor", b, "HEAD"])
            .ok_codes(&[0, 1])
            .run()?;
        if ancestor.code != 0 {
            return Err(AppError::InvalidInput("the commit is not in the current branch".into()));
        }
    }
    let r = range(base);
    let (log, (unpushed, listed)) = std::thread::scope(|s| {
        let unpushed = s.spawn(|| Git::new(repo).args(["rev-list", &r, "--not", "--remotes"]).run_string());
        // The commits of the range as git itself lists them: a message can't add one.
        let listed = s.spawn(|| Git::new(repo).args(["rev-list", &r]).run_string());
        let log = Git::new(repo)
            .args([
                "log",
                "--reverse",
                "--format=%H%x1f%P%x1f%an%x1f%at%x1f%s%x1f%B%x1e",
                &r,
                "--",
            ])
            .run();
        (log, (super::join(unpushed), super::join(listed)))
    });
    let unpushed: HashSet<String> = unpushed?.lines().map(String::from).collect();
    let listed: HashSet<String> = listed?.lines().map(String::from).collect();
    let mut commits: Vec<PlanCommit> = Vec::new();
    for record in decode(&log?.stdout).split(RECORD) {
        let record = record.trim_start_matches('\n');
        let f: Vec<&str> = record.splitn(6, FIELD).collect();
        let [hash, parents, author, time, subject, message] = f[..] else {
            continue;
        };
        // Control characters in a message could fake a record; only real commits count.
        if !listed.contains(hash) || commits.iter().any(|c| c.hash == hash) {
            continue;
        }
        if parents.split_whitespace().count() > 1 {
            return Err(AppError::MergesInRange);
        }
        commits.push(PlanCommit {
            hash: hash.to_string(),
            subject: subject.to_string(),
            message: message.trim_end().to_string(),
            author_name: author.to_string(),
            author_time: time.parse().unwrap_or(0),
            pushed: !unpushed.contains(hash),
        });
        if commits.len() > MAX_COMMITS {
            return Err(AppError::InvalidInput(format!("more than {MAX_COMMITS} commits")));
        }
    }
    Ok(commits)
}

/// Stores `bytes` as a blob and returns its id.
fn blob(repo: &Path, bytes: impl Into<Vec<u8>>) -> Result<String> {
    Ok(Git::new(repo)
        .args(["hash-object", "-w", "--stdin"])
        .stdin(bytes)
        .run_string()?
        .trim()
        .to_string())
}

/// Rewrites the commits after `base` as `steps` says (oldest first; every commit
/// of the range exactly once). May stop on conflicts like any rebase.
pub fn run(repo: &Path, base: Option<&str>, steps: &[Step]) -> Result<Outcome> {
    if operation::current(repo)?.is_some() {
        return Err(AppError::GitFailed(
            "another merge, rebase, revert or cherry-pick is in progress".into(),
        ));
    }
    let commits = plan(repo, base)?;
    let wanted: HashSet<&str> = commits.iter().map(|c| c.hash.as_str()).collect();
    let given: HashSet<&str> = steps.iter().map(|s| s.hash.as_str()).collect();
    if wanted != given || steps.len() != commits.len() {
        return Err(AppError::InvalidInput("the commits have changed; reload them".into()));
    }
    if commits.iter().any(|c| c.pushed) {
        return Err(AppError::AlreadyPushed);
    }
    match steps.iter().find(|s| s.action != Action::Drop) {
        Some(first) if matches!(first.action, Action::Squash | Action::Fixup) => {
            return Err(AppError::InvalidInput(
                "the oldest commit has nothing to be squashed into".into(),
            ));
        }
        None if !steps.is_empty() => {
            return Err(AppError::InvalidInput("every commit would be dropped".into()));
        }
        _ => {}
    }
    let unchanged = steps
        .iter()
        .zip(&commits)
        .all(|(s, c)| s.hash == c.hash && s.action == Action::Pick);
    if unchanged {
        return Ok(Outcome::Done);
    }

    let mut todo = String::new();
    for step in steps {
        let verb = match step.action {
            Action::Pick | Action::Reword => "pick",
            Action::Squash => "squash",
            Action::Fixup => "fixup",
            Action::Drop => "drop",
        };
        todo.push_str(&format!("{verb} {}\n", step.hash));
        if step.action == Action::Reword {
            let message = step.message.as_deref().map(str::trim).unwrap_or_default();
            if message.is_empty() {
                return Err(AppError::EmptySummary);
            }
            let oid = blob(repo, format!("{message}\n"))?;
            todo.push_str(&format!(
                "exec git cat-file blob {oid} | git commit --amend --only --allow-empty --cleanup=whitespace --quiet --file=-\n"
            ));
        }
    }
    let todo_oid = blob(repo, todo)?;
    let editor = format!("git cat-file blob {todo_oid} >");
    let mut git = Git::new(repo)
        .envs([("GIT_SEQUENCE_EDITOR".to_string(), editor)])
        // `keep`: a pick that turns empty must not vanish, or the reword after it would amend the commit before.
        .args(["rebase", "-i", "--autostash", "--empty=keep"]);
    git = match base {
        Some(b) => git.args(["--end-of-options", b]),
        None => git.arg("--root"),
    };
    operation::run_stoppable(repo, git)
}

#[cfg(test)]
#[path = "rebase_tests.rs"]
mod tests;
