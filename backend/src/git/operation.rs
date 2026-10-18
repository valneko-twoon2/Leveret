//! Multi-step operations that can stop on conflicts: merge, rebase, revert and
//! cherry-pick, plus continuing, skipping or aborting them.

use std::path::Path;

use serde::Serialize;

use super::{Git, PathspecFile, decode, git_dir};
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationKind {
    Merge,
    Rebase,
    Revert,
    CherryPick,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub kind: OperationKind,
    /// For a rebase: the branch being rebased (short name).
    pub branch: Option<String>,
    /// For a rebase: the current step and the number of steps.
    pub step: Option<u32>,
    pub total: Option<u32>,
    /// An interactive rebase (skipping a step would apply its later steps to
    /// the wrong commit, so it isn't offered).
    pub interactive: bool,
}

/// Result of a command that may stop on conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Done,
    Conflicts,
}

/// The operation in progress, if any.
pub fn current(repo: &Path) -> Result<Option<Operation>> {
    Ok(current_in(&git_dir(repo)?))
}

/// [`current`] for an already resolved git directory.
pub fn current_in(dir: &Path) -> Option<Operation> {
    let read = |p: &Path| std::fs::read_to_string(p).ok().map(|s| s.trim().to_string());
    for rebase_dir in ["rebase-merge", "rebase-apply"] {
        let d = dir.join(rebase_dir);
        // `git am` also uses rebase-apply; it is not something Leveret starts.
        if d.is_dir() && !d.join("applying").exists() {
            let branch = read(&d.join("head-name")).map(|h| h.trim_start_matches("refs/heads/").to_string());
            let num = |names: [&str; 2]| names.iter().find_map(|n| read(&d.join(n))?.parse().ok());
            return Some(Operation {
                kind: OperationKind::Rebase,
                branch: branch.filter(|b| b != "detached HEAD"),
                step: num(["msgnum", "next"]),
                total: num(["end", "last"]),
                interactive: d.join("interactive").exists(),
            });
        }
    }
    let kind = if dir.join("MERGE_HEAD").is_file() {
        OperationKind::Merge
    } else if dir.join("REVERT_HEAD").is_file() {
        OperationKind::Revert
    } else if dir.join("CHERRY_PICK_HEAD").is_file() {
        OperationKind::CherryPick
    } else {
        return None;
    };
    Some(Operation {
        kind,
        branch: None,
        step: None,
        total: None,
        interactive: false,
    })
}

/// Paths with unresolved conflicts.
pub fn conflicted_paths(repo: &Path) -> Result<Vec<String>> {
    let out = Git::new(repo)
        .args(["diff", "--name-only", "-z", "--diff-filter=U"])
        .run()?;
    Ok(decode(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect())
}

/// Runs a command that may stop on conflicts. A failure that leaves an
/// operation in progress is reported as [`Outcome::Conflicts`]; the repository
/// state, not the error text, is what decides.
pub fn run_stoppable(repo: &Path, git: Git<'_>) -> Result<Outcome> {
    match git.run() {
        Ok(_) => Ok(Outcome::Done),
        Err(e @ AppError::GitFailed(_)) => match current(repo)? {
            Some(_) if !conflicted_paths(repo)?.is_empty() => Ok(Outcome::Conflicts),
            _ => Err(e.classify_git()),
        },
        Err(e) => Err(e),
    }
}

fn require_none(repo: &Path) -> Result<()> {
    match current(repo)? {
        Some(_) => Err(AppError::GitFailed(
            "another merge, rebase, revert or cherry-pick is in progress".into(),
        )),
        None => Ok(()),
    }
}

/// Reverts `hash` with a new commit. Merge commits are reverted against their first parent.
pub fn revert(repo: &Path, hash: &str, is_merge: bool) -> Result<Outcome> {
    require_none(repo)?;
    let mut git = Git::new(repo).args(["revert", "--no-edit"]);
    if is_merge {
        git = git.args(["-m", "1"]);
    }
    run_stoppable(repo, git.arg("--end-of-options").arg(hash))
}

/// Applies the changes of `hash` to the current branch as a new commit.
pub fn cherry_pick(repo: &Path, hash: &str, is_merge: bool) -> Result<Outcome> {
    require_none(repo)?;
    let mut git = Git::new(repo).arg("cherry-pick");
    if is_merge {
        git = git.args(["-m", "1"]);
    }
    run_stoppable(repo, git.arg("--end-of-options").arg(hash))
}

/// Rebases the current branch onto `onto` (a full ref name). Local changes are
/// stashed and restored automatically.
pub fn rebase(repo: &Path, onto: &str) -> Result<Outcome> {
    require_none(repo)?;
    let git = Git::new(repo).args(["rebase", "--autostash", "--end-of-options", onto]);
    run_stoppable(repo, git)
}

/// Merges `branch` (a full ref name) into the current branch.
pub fn merge(repo: &Path, branch: &str) -> Result<Outcome> {
    require_none(repo)?;
    let git = Git::new(repo).args(["merge", "--no-edit", "--end-of-options", branch]);
    run_stoppable(repo, git)
}

/// Marks conflicted files as resolved (stages them as they are in the working tree).
pub fn mark_resolved(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Err(AppError::NothingSelected);
    }
    let spec = PathspecFile::new(paths)?;
    Git::new(repo).args(["add", "--all"]).args(spec.args()).run()?;
    Ok(())
}

/// Concludes the current step once every conflict is resolved.
pub fn continue_(repo: &Path) -> Result<Outcome> {
    let op = current(repo)?.ok_or(AppError::NoOperation)?;
    if !conflicted_paths(repo)?.is_empty() {
        return Err(AppError::ConflictsRemain);
    }
    // Resolved files that were edited after "mark as resolved" belong to the step too.
    Git::new(repo).args(["add", "--update"]).run()?;
    let git = Git::new(repo);
    let git = match op.kind {
        OperationKind::Merge => git.args(["commit", "--no-edit"]),
        OperationKind::Rebase => git.args(["rebase", "--continue"]),
        OperationKind::Revert => git.args(["revert", "--continue"]),
        OperationKind::CherryPick => git.args(["cherry-pick", "--continue"]),
    };
    run_stoppable(repo, git)
}

/// Skips the current commit of a rebase, revert or cherry-pick.
pub fn skip(repo: &Path) -> Result<Outcome> {
    let op = current(repo)?.ok_or(AppError::NoOperation)?;
    let git = Git::new(repo);
    let git = match op.kind {
        OperationKind::Merge => return Err(AppError::NoOperation),
        OperationKind::Rebase => git.args(["rebase", "--skip"]),
        OperationKind::Revert => git.args(["revert", "--skip"]),
        OperationKind::CherryPick => git.args(["cherry-pick", "--skip"]),
    };
    run_stoppable(repo, git)
}

/// Cancels the operation and returns to the state before it started.
pub fn abort(repo: &Path) -> Result<()> {
    let op = current(repo)?.ok_or(AppError::NoOperation)?;
    let sub = match op.kind {
        OperationKind::Merge => "merge",
        OperationKind::Rebase => "rebase",
        OperationKind::Revert => "revert",
        OperationKind::CherryPick => "cherry-pick",
    };
    Git::new(repo)
        .args([sub, "--abort"])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{self, git, git_out};

    fn repo() -> tempfile::TempDir {
        let dir = testutil::repo();
        let p = dir.path();
        std::fs::write(p.join("a.txt"), "base\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "base"]);
        dir
    }

    /// main and feature both change a.txt, so combining them conflicts.
    fn diverged() -> tempfile::TempDir {
        let dir = repo();
        let p = dir.path();
        git(p, &["switch", "-qc", "feature"]);
        std::fs::write(p.join("a.txt"), "feature\n").unwrap();
        git(p, &["commit", "-qam", "feature"]);
        std::fs::write(p.join("f.txt"), "f\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "feature 2"]);
        git(p, &["switch", "-q", "main"]);
        std::fs::write(p.join("a.txt"), "main\n").unwrap();
        git(p, &["commit", "-qam", "main"]);
        git(p, &["switch", "-q", "feature"]);
        dir
    }

    #[test]
    fn rebase_with_conflict_then_continue() {
        let dir = diverged();
        let p = dir.path();
        assert_eq!(rebase(p, "refs/heads/main").unwrap(), Outcome::Conflicts);
        let op = current(p).unwrap().unwrap();
        assert_eq!(op.kind, OperationKind::Rebase);
        assert_eq!(op.branch.as_deref(), Some("feature"));
        assert_eq!((op.step, op.total), (Some(1), Some(2)));

        assert!(matches!(continue_(p), Err(AppError::ConflictsRemain)));
        std::fs::write(p.join("a.txt"), "resolved\n").unwrap();
        mark_resolved(p, &["a.txt".into()]).unwrap();
        assert_eq!(continue_(p).unwrap(), Outcome::Done);
        assert!(current(p).unwrap().is_none());

        let log = git_out(p, &["log", "--format=%s"]);
        assert_eq!(
            log.lines().collect::<Vec<_>>(),
            ["feature 2", "feature", "main", "base"]
        );
    }

    #[test]
    fn rebase_abort_restores_branch() {
        let dir = diverged();
        let p = dir.path();
        let before = git_out(p, &["rev-parse", "HEAD"]);
        assert_eq!(rebase(p, "refs/heads/main").unwrap(), Outcome::Conflicts);
        abort(p).unwrap();
        assert!(current(p).unwrap().is_none());
        assert_eq!(git_out(p, &["rev-parse", "HEAD"]), before);
        assert!(matches!(abort(p), Err(AppError::NoOperation)));
    }

    #[test]
    fn rebase_skip_drops_conflicting_commit() {
        let dir = diverged();
        let p = dir.path();
        assert_eq!(rebase(p, "refs/heads/main").unwrap(), Outcome::Conflicts);
        assert_eq!(skip(p).unwrap(), Outcome::Done);
        let log = git_out(p, &["log", "--format=%s"]);
        assert_eq!(log.lines().collect::<Vec<_>>(), ["feature 2", "main", "base"]);
    }

    #[test]
    fn rebase_keeps_local_changes() {
        let dir = repo();
        let p = dir.path();
        git(p, &["switch", "-qc", "feature"]);
        std::fs::write(p.join("f.txt"), "f\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "feature"]);
        git(p, &["switch", "-q", "main"]);
        std::fs::write(p.join("a.txt"), "main\n").unwrap();
        git(p, &["commit", "-qam", "main"]);
        git(p, &["switch", "-q", "feature"]);

        std::fs::write(p.join("f.txt"), "f\nwip\n").unwrap();
        assert_eq!(rebase(p, "refs/heads/main").unwrap(), Outcome::Done);
        assert_eq!(std::fs::read_to_string(p.join("f.txt")).unwrap(), "f\nwip\n");
        assert_eq!(std::fs::read_to_string(p.join("a.txt")).unwrap(), "main\n");
    }

    #[test]
    fn merge_conflict_then_continue_creates_merge_commit() {
        let dir = diverged();
        let p = dir.path();
        assert_eq!(merge(p, "refs/heads/main").unwrap(), Outcome::Conflicts);
        assert_eq!(current(p).unwrap().unwrap().kind, OperationKind::Merge);
        assert_eq!(conflicted_paths(p).unwrap(), ["a.txt"]);
        std::fs::write(p.join("a.txt"), "both\n").unwrap();
        mark_resolved(p, &["a.txt".into()]).unwrap();
        assert_eq!(continue_(p).unwrap(), Outcome::Done);
        let parents = git_out(p, &["log", "-1", "--format=%P"]);
        assert_eq!(parents.split_whitespace().count(), 2);
    }

    #[test]
    fn revert_commit_and_revert_conflict() {
        let dir = repo();
        let p = dir.path();
        std::fs::write(p.join("b.txt"), "b\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "add b"]);
        let hash = git_out(p, &["rev-parse", "HEAD"]);
        assert_eq!(revert(p, &hash, false).unwrap(), Outcome::Done);
        assert!(!p.join("b.txt").exists());
        let subject = git_out(p, &["log", "-1", "--format=%s"]);
        assert!(subject.starts_with("Revert \"add b\""));

        // Reverting a change that was edited again afterwards conflicts.
        std::fs::write(p.join("a.txt"), "two\n").unwrap();
        git(p, &["commit", "-qam", "two"]);
        let two = git_out(p, &["rev-parse", "HEAD"]);
        std::fs::write(p.join("a.txt"), "three\n").unwrap();
        git(p, &["commit", "-qam", "three"]);
        assert_eq!(revert(p, &two, false).unwrap(), Outcome::Conflicts);
        assert_eq!(current(p).unwrap().unwrap().kind, OperationKind::Revert);
        abort(p).unwrap();
        assert!(current(p).unwrap().is_none());
    }

    #[test]
    fn revert_refuses_to_overwrite_local_changes() {
        let dir = repo();
        let p = dir.path();
        std::fs::write(p.join("a.txt"), "two\n").unwrap();
        git(p, &["commit", "-qam", "two"]);
        let two = git_out(p, &["rev-parse", "HEAD"]);
        std::fs::write(p.join("a.txt"), "local\n").unwrap();
        let err = revert(p, &two, false).unwrap_err();
        assert!(matches!(err, AppError::LocalChangesConflict(_)), "{err:?}");
        assert_eq!(std::fs::read_to_string(p.join("a.txt")).unwrap(), "local\n");
    }
}
