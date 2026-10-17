//! Fetch, pull, push and sync with the remote.

use std::path::Path;

use super::operation::{self, Outcome};
use super::{Git, Phases, Progress, decode, progress_reporter};
use crate::error::{AppError, Result};

const FETCH_PHASES: &Phases = &[
    ("Counting objects", 0, 5),
    ("Compressing objects", 5, 10),
    ("Receiving objects", 10, 85),
    ("Resolving deltas", 85, 100),
];

const PULL_PHASES: &Phases = &[
    ("Counting objects", 0, 5),
    ("Compressing objects", 5, 10),
    ("Receiving objects", 10, 75),
    ("Resolving deltas", 75, 90),
    ("Updating files", 90, 100),
];

const PUSH_PHASES: &Phases = &[
    ("Enumerating objects", 0, 5),
    ("Counting objects", 5, 10),
    ("Compressing objects", 10, 20),
    ("Writing objects", 20, 100),
];

fn config(repo: &Path, key: &str) -> Result<Option<String>> {
    let out = Git::new(repo).args(["config", "--get", key]).ok_codes(&[0, 1]).run()?;
    let value = decode(&out.stdout).trim().to_string();
    Ok(Some(value).filter(|v| out.code == 0 && !v.is_empty()))
}

/// Names of the configured remotes.
pub fn remotes(repo: &Path) -> Result<Vec<String>> {
    Ok(Git::new(repo)
        .arg("remote")
        .run_string()?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// URLs of all remotes (fetch and push URLs), to decide which credentials git needs.
pub fn remote_urls(repo: &Path) -> Result<Vec<String>> {
    let out = Git::new(repo)
        .args(["config", "--get-regexp", r"^remote\..*\.(push)?url$"])
        .ok_codes(&[0, 1])
        .run()?;
    Ok(decode(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once(' '))
        .map(|(_, url)| url.to_string())
        .collect())
}

fn current_branch(repo: &Path) -> Result<String> {
    let out = Git::new(repo)
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .ok_codes(&[0, 1])
        .run()?;
    let name = decode(&out.stdout).trim().to_string();
    if out.code != 0 || name.is_empty() {
        return Err(AppError::DetachedHead);
    }
    Ok(name)
}

/// `(remote, merge ref)` of the branch's upstream, e.g. `("origin", "refs/heads/main")`.
fn upstream(repo: &Path, branch: &str) -> Result<Option<(String, String)>> {
    let remote = config(repo, &format!("branch.{branch}.remote"))?;
    let merge = config(repo, &format!("branch.{branch}.merge"))?;
    Ok(remote.zip(merge).filter(|(r, _)| r != "."))
}

/// Remote used to publish a branch that has no upstream yet.
fn push_remote(repo: &Path, branch: &str) -> Result<String> {
    if let Some(r) = config(repo, &format!("branch.{branch}.pushRemote"))?.or(config(repo, "remote.pushDefault")?) {
        return Ok(r);
    }
    default_remote(repo)
}

/// `origin`, or the first remote when there is no `origin`.
pub fn default_remote(repo: &Path) -> Result<String> {
    let remotes = remotes(repo)?;
    match remotes.iter().find(|r| *r == "origin").or(remotes.first()) {
        Some(r) => Ok(r.clone()),
        None => Err(AppError::NoRemote),
    }
}

/// Fetches all remotes and prunes deleted remote branches.
pub fn fetch(repo: &Path, envs: Vec<(String, String)>, on_progress: impl FnMut(Progress)) -> Result<()> {
    if remotes(repo)?.is_empty() {
        return Err(AppError::NoRemote);
    }
    Git::new(repo)
        .envs(envs)
        .args(["fetch", "--all", "--prune", "--progress"])
        .run_with_progress(progress_reporter(FETCH_PHASES, on_progress))
        .map_err(AppError::classify_git)
}

/// Pulls the upstream of the current branch. Merges unless the user configured
/// `pull.rebase`; stops with [`Outcome::Conflicts`] when that conflicts.
pub fn pull(repo: &Path, envs: Vec<(String, String)>, on_progress: impl FnMut(Progress)) -> Result<Outcome> {
    let branch = current_branch(repo)?;
    if upstream(repo, &branch)?.is_none() {
        return Err(AppError::NoUpstream);
    }
    let mut git = Git::new(repo).envs(envs);
    git = git.args(["pull", "--progress", "--no-edit"]);
    if config(repo, "pull.rebase")?.is_none() && config(repo, &format!("branch.{branch}.rebase"))?.is_none() {
        // Without this, recent git refuses to pull diverged branches until configured.
        git = git.arg("--no-rebase");
    }
    match git.run_with_progress(progress_reporter(PULL_PHASES, on_progress)) {
        Ok(()) => Ok(Outcome::Done),
        Err(e @ AppError::GitFailed(_)) => match operation::current(repo)? {
            Some(_) if !operation::conflicted_paths(repo)?.is_empty() => Ok(Outcome::Conflicts),
            _ => Err(e.classify_git()),
        },
        Err(e) => Err(e),
    }
}

/// Pushes the current branch to its upstream, or publishes it (setting the
/// upstream) when it has none.
pub fn push(repo: &Path, envs: Vec<(String, String)>, on_progress: impl FnMut(Progress)) -> Result<()> {
    let branch = current_branch(repo)?;
    let git = Git::new(repo).envs(envs).args(["push", "--progress"]);
    let git = match upstream(repo, &branch)? {
        Some((remote, merge)) => git.args(["--", &remote]).arg(format!("refs/heads/{branch}:{merge}")),
        None => {
            let remote = push_remote(repo, &branch)?;
            git.args(["--set-upstream", "--", &remote])
                .arg(format!("refs/heads/{branch}:refs/heads/{branch}"))
        }
    };
    git.run_with_progress(progress_reporter(PUSH_PHASES, on_progress))
        .map_err(AppError::classify_git)
}

/// Pull, then push. A branch without upstream is just published.
pub fn sync(repo: &Path, envs: Vec<(String, String)>, mut on_progress: impl FnMut(Progress)) -> Result<Outcome> {
    let branch = current_branch(repo)?;
    if upstream(repo, &branch)?.is_some() && pull(repo, envs.clone(), &mut on_progress)? == Outcome::Conflicts {
        return Ok(Outcome::Conflicts);
    }
    push(repo, envs, on_progress)?;
    Ok(Outcome::Done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::status;
    use crate::testutil::{self, git};

    /// A bare "remote", a repository that pushed one commit to it, and a clone of it.
    fn setup() -> (tempfile::TempDir, tempfile::TempDir, tempfile::TempDir) {
        let remote = testutil::bare();
        let url = remote.path().display().to_string();
        let a = testutil::repo_with_commit();
        git(a.path(), &["remote", "add", "origin", &url]);
        git(a.path(), &["push", "-q", "-u", "origin", "main"]);
        // The clone gets the same identity as the template in one git process.
        let b = tempfile::tempdir().unwrap();
        git(
            b.path(),
            &[
                "clone",
                "-q",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "core.autocrlf=false",
                "--",
                &url,
                ".",
            ],
        );
        (remote, a, b)
    }

    #[test]
    fn push_fetch_pull() {
        let (_remote, a, b) = setup();
        std::fs::write(a.path().join("b.txt"), "b\n").unwrap();
        git(a.path(), &["add", "."]);
        git(a.path(), &["commit", "-qm", "second"]);
        assert_eq!(status::get(a.path()).unwrap().ahead, 1);
        push(a.path(), Vec::new(), |_| {}).unwrap();
        assert_eq!(status::get(a.path()).unwrap().ahead, 0);

        fetch(b.path(), Vec::new(), |_| {}).unwrap();
        assert_eq!(status::get(b.path()).unwrap().behind, 1);
        assert_eq!(pull(b.path(), Vec::new(), |_| {}).unwrap(), Outcome::Done);
        assert!(b.path().join("b.txt").exists());
    }

    #[test]
    fn push_rejected_then_sync_with_conflict() {
        let (_remote, a, b) = setup();
        std::fs::write(a.path().join("a.txt"), "from a\n").unwrap();
        git(a.path(), &["commit", "-qam", "a"]);
        push(a.path(), Vec::new(), |_| {}).unwrap();

        std::fs::write(b.path().join("a.txt"), "from b\n").unwrap();
        git(b.path(), &["commit", "-qam", "b"]);
        let err = push(b.path(), Vec::new(), |_| {}).unwrap_err();
        assert!(matches!(err, AppError::PushRejected(_)), "{err:?}");

        assert_eq!(sync(b.path(), Vec::new(), |_| {}).unwrap(), Outcome::Conflicts);
        assert_eq!(
            operation::current(b.path()).unwrap().unwrap().kind,
            operation::OperationKind::Merge
        );
    }

    #[test]
    fn publish_new_branch() {
        let (_remote, a, _b) = setup();
        git(a.path(), &["switch", "-qc", "topic"]);
        let st = status::get(a.path()).unwrap();
        assert!(st.upstream.is_none());
        assert!(matches!(pull(a.path(), Vec::new(), |_| {}), Err(AppError::NoUpstream)));
        sync(a.path(), Vec::new(), |_| {}).unwrap();
        assert_eq!(status::get(a.path()).unwrap().upstream.as_deref(), Some("origin/topic"));
    }

    #[test]
    fn errors_without_remote_or_branch() {
        let dir = testutil::repo_with_commit();
        let p = dir.path();
        assert!(matches!(fetch(p, Vec::new(), |_| {}), Err(AppError::NoRemote)));
        assert!(matches!(push(p, Vec::new(), |_| {}), Err(AppError::NoRemote)));
        git(p, &["switch", "-q", "--detach"]);
        assert!(matches!(push(p, Vec::new(), |_| {}), Err(AppError::DetachedHead)));
    }
}
