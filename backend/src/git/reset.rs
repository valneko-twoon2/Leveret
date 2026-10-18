//! Moving the current branch to another commit, and bringing one file back to
//! how it was in a commit (or undoing what a commit did to it).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::status::ChangeKind;
use super::{Git, decode, empty_tree, operation, status};
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResetMode {
    /// The commits leave the branch; their changes stay as uncommitted changes.
    Keep,
    /// The commits leave the branch and their changes are thrown away.
    Discard,
}

/// What resetting to a commit would do, for the confirmation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetInfo {
    /// Commits that would leave the branch.
    pub count: u32,
    /// Of those, commits that are on a remote: pushing afterwards needs a force push.
    pub pushed: u32,
    /// There are uncommitted changes to tracked files (a discarding reset would lose them).
    pub dirty: bool,
}

fn count(repo: &Path, args: &[&str]) -> Result<u32> {
    let out = Git::new(repo).args(["rev-list", "--count"]).args(args).run_string()?;
    Ok(out.trim().parse().unwrap_or(0))
}

fn tracked_changes(repo: &Path) -> Result<bool> {
    Ok(status::get_with(repo, false)?
        .files
        .iter()
        .any(|f| f.kind != ChangeKind::Untracked))
}

pub fn reset_info(repo: &Path, target: &str) -> Result<ResetInfo> {
    let range = format!("{target}..HEAD");
    let total = count(repo, &[&range])?;
    let unpushed = count(repo, &[&range, "--not", "--remotes"])?;
    Ok(ResetInfo {
        count: total,
        pushed: total.saturating_sub(unpushed),
        dirty: tracked_changes(repo)?,
    })
}

/// Moves the current branch (or a detached HEAD) to `target`. Discarding is
/// refused while there are uncommitted changes, so it never loses work that
/// isn't in a commit (the removed commits stay reachable through the reflog).
pub fn reset(repo: &Path, target: &str, mode: ResetMode) -> Result<()> {
    if operation::current(repo)?.is_some() {
        return Err(AppError::GitFailed(
            "a merge, rebase, revert or cherry-pick is in progress".into(),
        ));
    }
    let flag = match mode {
        ResetMode::Keep => "--soft",
        ResetMode::Discard => {
            if tracked_changes(repo)? {
                return Err(AppError::LocalChangesConflict(
                    "commit or stash the changes first".into(),
                ));
            }
            "--hard"
        }
    };
    Git::new(repo)
        .args(["reset", "-q", flag, target, "--"])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Copies the current file (if any) into a temporary folder and hands the copy
/// to `remove` (the Recycle Bin), so overwriting it can be undone.
fn back_up(repo: &Path, path: &str, remove: impl FnOnce(&[PathBuf]) -> Result<()>) -> Result<()> {
    let file = repo.join(path);
    if !file.is_file() {
        return Ok(());
    }
    let dir = tempfile::tempdir()?;
    let name = file.file_name().ok_or_else(|| AppError::InvalidInput(path.into()))?;
    let copy = dir.path().join(name);
    std::fs::copy(&file, &copy)?;
    remove(&[copy])
}

fn recycle(files: &[PathBuf]) -> Result<()> {
    trash::delete_all(files).map_err(|e| AppError::Other(e.to_string()))
}

/// Puts `path` in the working tree back to how it is in commit `rev`.
pub fn restore_file(repo: &Path, rev: &str, path: &str) -> Result<()> {
    restore_file_with(repo, rev, path, recycle)
}

pub(crate) fn restore_file_with(
    repo: &Path,
    rev: &str,
    path: &str,
    remove: impl FnOnce(&[PathBuf]) -> Result<()>,
) -> Result<()> {
    let exists = Git::new(repo)
        .args(["cat-file", "-e"])
        .arg(format!("{rev}:{path}"))
        .ok_codes(&[0, 1, 128])
        .run()?;
    if exists.code != 0 {
        return Err(AppError::InvalidInput(format!("'{path}' is not in that commit")));
    }
    back_up(repo, path, remove)?;
    Git::new(repo)
        .args(["restore", "--worktree"])
        .arg(format!("--source={rev}"))
        .args(["--", path])
        .run()?;
    Ok(())
}

/// Undoes what commit `rev` did to `path` (and its old name, for a rename), on
/// top of the file as it is now. Fails when later changes overlap.
pub fn undo_file_change(repo: &Path, rev: &str, path: &str, orig_path: Option<&str>) -> Result<()> {
    undo_file_change_with(repo, rev, path, orig_path, recycle)
}

pub(crate) fn undo_file_change_with(
    repo: &Path,
    rev: &str,
    path: &str,
    orig_path: Option<&str>,
    remove: impl FnOnce(&[PathBuf]) -> Result<()>,
) -> Result<()> {
    let parent = Git::new(repo)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{rev}^1"))
        .ok_codes(&[0, 1])
        .run()?;
    let parent = match parent.code {
        0 => decode(&parent.stdout).trim().to_string(),
        _ => empty_tree(repo)?,
    };
    let patch = Git::new(repo)
        .args(["diff", "--binary", "--no-color", "--no-ext-diff", "--no-textconv", "-M"])
        .args(["--src-prefix=a/", "--dst-prefix=b/", &parent, rev, "--", path])
        .args(orig_path)
        .run()?
        .stdout;
    if patch.is_empty() {
        return Err(AppError::NoChanges);
    }
    let check = Git::new(repo)
        .args(["apply", "-R", "--check", "--whitespace=nowarn", "-"])
        .stdin(patch.clone())
        .ok_codes(&[0, 1])
        .run()?;
    if check.code != 0 {
        return Err(AppError::ChangesDontApply(path.into()));
    }
    back_up(repo, path, remove)?;
    Git::new(repo)
        .args(["apply", "-R", "--whitespace=nowarn", "-"])
        .stdin(patch)
        .run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, git_out, repo};

    /// Ten lines: c1 changes line 2, c2 changes line 9 (far enough apart for
    /// separate hunks). Everything up to c1 is "pushed".
    fn lines(two: &str, nine: &str) -> String {
        (1..=10)
            .map(|i| match i {
                2 => format!("{two}\n"),
                9 => format!("{nine}\n"),
                _ => format!("{i}\n"),
            })
            .collect()
    }

    fn history() -> tempfile::TempDir {
        let dir = repo();
        let p = dir.path();
        for (i, text) in [lines("2", "9"), lines("two", "9"), lines("two", "nine")]
            .iter()
            .enumerate()
        {
            std::fs::write(p.join("f.txt"), text).unwrap();
            git(p, &["add", "."]);
            git(p, &["commit", "-qm", &format!("c{i}")]);
        }
        git(p, &["update-ref", "refs/remotes/origin/main", "HEAD~1"]);
        dir
    }

    fn read(p: &Path) -> String {
        std::fs::read_to_string(p.join("f.txt")).unwrap()
    }

    #[test]
    fn reset_keeps_or_discards_the_changes() {
        let dir = history();
        let p = dir.path();
        let first = git_out(p, &["rev-parse", "HEAD~2"]);
        let info = reset_info(p, &first).unwrap();
        assert_eq!((info.count, info.pushed, info.dirty), (2, 1, false));

        reset(p, &first, ResetMode::Keep).unwrap();
        assert_eq!(git_out(p, &["log", "-1", "--format=%s"]), "c0");
        assert_eq!(read(p), lines("two", "nine"));
        assert!(reset_info(p, &first).unwrap().dirty);
        // Discarding is refused while that would lose the changes.
        let err = reset(p, &first, ResetMode::Discard).unwrap_err();
        assert!(matches!(err, AppError::LocalChangesConflict(_)), "{err:?}");

        git(p, &["commit", "-qam", "again"]);
        reset(p, &first, ResetMode::Discard).unwrap();
        assert_eq!(read(p), lines("2", "9"));
        assert!(status::get(p).unwrap().files.is_empty());
    }

    #[test]
    fn restores_a_file_and_undoes_a_commit_s_change() {
        let dir = history();
        let p = dir.path();
        std::fs::write(p.join("f.txt"), lines("local", "nine")).unwrap();
        let first = git_out(p, &["rev-parse", "HEAD~2"]);
        let backups = std::cell::RefCell::new(Vec::new());
        let keep = |files: &[PathBuf]| {
            backups.borrow_mut().push(std::fs::read_to_string(&files[0]).unwrap());
            Ok(())
        };
        restore_file_with(p, &first, "f.txt", keep).unwrap();
        assert_eq!(read(p), lines("2", "9"));
        assert!(backups.borrow()[0].contains("local"));
        assert!(restore_file_with(p, &first, "missing.txt", |_| Ok(())).is_err());

        // Undo c1 (line 2) on top of the current content, keeping c2 (line 9).
        git(p, &["checkout", "--", "f.txt"]);
        let second = git_out(p, &["rev-parse", "HEAD~1"]);
        undo_file_change_with(p, &second, "f.txt", None, |_| Ok(())).unwrap();
        assert_eq!(read(p), lines("2", "nine"));
        // Undoing it again no longer applies.
        let err = undo_file_change_with(p, &second, "f.txt", None, |_| Ok(())).unwrap_err();
        assert!(matches!(err, AppError::ChangesDontApply(_)), "{err:?}");
    }
}
