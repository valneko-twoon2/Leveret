//! Listing, creating, switching, renaming and deleting branches, and their upstreams.

use std::path::Path;

use serde::Serialize;

use super::{Git, decode, remote};
use crate::error::{AppError, Result};

const FIELD: char = '\x1f';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BranchKind {
    Local,
    Remote,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    /// Short name: `main`, `origin/main`.
    pub name: String,
    /// Full ref name: `refs/heads/main`, `refs/remotes/origin/main`.
    pub full_ref: String,
    pub kind: BranchKind,
    /// The branch HEAD points at.
    pub current: bool,
    /// Upstream of a local branch (`origin/main`).
    pub upstream: Option<String>,
    /// The upstream branch was deleted on the remote.
    pub upstream_gone: bool,
    pub ahead: u32,
    pub behind: u32,
    pub oid: String,
    /// Unix seconds of the tip commit.
    pub time: i64,
    pub subject: String,
}

pub fn list(repo: &Path) -> Result<Vec<Branch>> {
    let format = [
        "%(refname)",
        "%(refname:short)",
        "%(HEAD)",
        "%(upstream:short)",
        "%(upstream:track,nobracket)",
        "%(objectname)",
        "%(committerdate:unix)",
        "%(symref)",
        "%(contents:subject)",
    ]
    .join("%1f");
    let out = Git::new(repo)
        .args(["for-each-ref", "--sort=-committerdate"])
        .arg(format!("--format={format}"))
        .args(["refs/heads", "refs/remotes"])
        .run()?;
    Ok(parse(&decode(&out.stdout)))
}

fn parse(text: &str) -> Vec<Branch> {
    let mut branches = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.splitn(9, FIELD).collect();
        let [full_ref, name, head, upstream, track, oid, time, symref, subject] = f[..] else {
            continue;
        };
        // Skip `origin/HEAD` and other symbolic refs.
        if !symref.is_empty() {
            continue;
        }
        let kind = if full_ref.starts_with("refs/heads/") {
            BranchKind::Local
        } else {
            BranchKind::Remote
        };
        let (mut ahead, mut behind) = (0, 0);
        for part in track.split(", ") {
            if let Some(n) = part.strip_prefix("ahead ") {
                ahead = n.parse().unwrap_or(0);
            } else if let Some(n) = part.strip_prefix("behind ") {
                behind = n.parse().unwrap_or(0);
            }
        }
        branches.push(Branch {
            name: name.into(),
            full_ref: full_ref.into(),
            kind,
            current: head == "*",
            upstream: Some(upstream).filter(|u| !u.is_empty()).map(String::from),
            upstream_gone: track == "gone",
            ahead,
            behind,
            oid: oid.into(),
            time: time.parse().unwrap_or(0),
            subject: subject.into(),
        });
    }
    branches
}

fn branch_exists(repo: &Path, name: &str) -> Result<bool> {
    let out = Git::new(repo)
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/heads/{name}"))
        .ok_codes(&[0, 1])
        .run()?;
    Ok(out.code == 0)
}

/// Creates branch `name` at `start` (a commit or full ref; HEAD when `None`),
/// switching to it when `checkout` is set. Local changes are carried over.
pub fn create(repo: &Path, name: &str, start: Option<&str>, checkout: bool) -> Result<()> {
    if branch_exists(repo, name)? {
        return Err(AppError::BranchExists(name.into()));
    }
    let mut git = if checkout {
        Git::new(repo).args(["switch", "--no-track", "-c", name])
    } else {
        Git::new(repo).args(["branch", "--no-track", name])
    };
    if let Some(start) = start {
        git = git.arg(start);
    }
    git.run().map_err(AppError::classify_git)?;
    Ok(())
}

/// Switches to a branch given as a full ref. A remote branch is checked out as a
/// local branch of the same name that tracks it (reusing an existing one).
pub fn switch(repo: &Path, full_ref: &str) -> Result<()> {
    let git = if let Some(local) = full_ref.strip_prefix("refs/heads/") {
        Git::new(repo).args(["switch", "--no-guess", local])
    } else if let Some(remote) = full_ref.strip_prefix("refs/remotes/") {
        let (_, local) = split_remote(repo, remote).ok_or_else(|| AppError::InvalidInput(full_ref.into()))?;
        if branch_exists(repo, &local)? {
            Git::new(repo).args(["switch", "--no-guess", &local])
        } else {
            Git::new(repo).args(["switch", "-c", &local, "--track", remote])
        }
    } else {
        return Err(AppError::InvalidInput(full_ref.into()));
    };
    git.run().map_err(AppError::classify_git)?;
    Ok(())
}

/// Checks out a commit without a branch (detached HEAD).
pub fn checkout_detached(repo: &Path, hash: &str) -> Result<()> {
    Git::new(repo)
        .args(["switch", "--detach", "--end-of-options", hash])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Deletes a local branch. Without `force`, git refuses when it isn't merged.
pub fn delete(repo: &Path, name: &str, force: bool) -> Result<()> {
    Git::new(repo)
        .args(["branch", if force { "-D" } else { "-d" }, "--", name])
        .run()
        .map_err(|e| match e {
            AppError::GitFailed(msg) if msg.contains("not fully merged") => AppError::BranchNotMerged(name.into()),
            e => e.classify_git(),
        })?;
    Ok(())
}

/// Renames a local branch (its upstream setting moves with it).
pub fn rename(repo: &Path, old: &str, new: &str) -> Result<()> {
    if !branch_exists(repo, old)? {
        return Err(AppError::InvalidInput(old.into()));
    }
    if old != new && branch_exists(repo, new)? {
        return Err(AppError::BranchExists(new.into()));
    }
    Git::new(repo)
        .args(["branch", "-m", "--", old, new])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Makes `upstream` (a full `refs/remotes/..` ref) the upstream of the local branch,
/// or removes the upstream when `None`.
pub fn set_upstream(repo: &Path, local: &str, upstream: Option<&str>) -> Result<()> {
    let option = match upstream {
        Some(u) => format!("--set-upstream-to={u}"),
        None => "--unset-upstream".to_string(),
    };
    Git::new(repo)
        .args(["branch", &option, "--", local])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Splits `origin/feature/x` into the configured remote (the longest match, as names
/// may contain `/`) and the branch name on it.
fn split_remote(repo: &Path, name: &str) -> Option<(String, String)> {
    let remotes = remote::remotes(repo).ok()?;
    remotes
        .iter()
        .filter_map(|r| {
            name.strip_prefix(r.as_str())
                .and_then(|b| b.strip_prefix('/'))
                .map(|b| (r, b))
        })
        .max_by_key(|(r, _)| r.len())
        .filter(|(_, b)| !b.is_empty())
        .map(|(r, b)| (r.clone(), b.to_string()))
}

/// Deletes a branch on its remote (`push --delete`); the remote is told apart from
/// the branch name by the configured remotes, as names may contain `/`.
pub fn delete_remote(repo: &Path, envs: Vec<(String, String)>, full_ref: &str) -> Result<()> {
    let name = full_ref
        .strip_prefix("refs/remotes/")
        .ok_or_else(|| AppError::InvalidInput(full_ref.into()))?;
    let (remote, branch) = split_remote(repo, name).ok_or_else(|| AppError::InvalidInput(full_ref.into()))?;
    Git::new(repo)
        .envs(envs)
        .args(["push", "--delete", "--", &remote])
        .arg(format!("refs/heads/{branch}"))
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, git_out, repo_with_commit as repo};

    #[test]
    fn parses_for_each_ref() {
        let text = "refs/heads/main\x1fmain\x1f*\x1forigin/main\x1fahead 2, behind 1\x1fabc\x1f100\x1f\x1finit\n\
                    refs/heads/old\x1fold\x1f \x1forigin/old\x1fgone\x1fdef\x1f90\x1f\x1fold work\n\
                    refs/remotes/origin/HEAD\x1forigin\x1f \x1f\x1f\x1fabc\x1f100\x1frefs/remotes/origin/main\x1finit\n\
                    refs/remotes/origin/main\x1forigin/main\x1f \x1f\x1f\x1fabc\x1f100\x1f\x1fa\x1fb\n";
        let b = parse(text);
        assert_eq!(b.len(), 3);
        assert!(b[0].current && b[0].kind == BranchKind::Local);
        assert_eq!((b[0].ahead, b[0].behind), (2, 1));
        assert!(b[1].upstream_gone);
        assert_eq!(b[2].kind, BranchKind::Remote);
        assert_eq!(b[2].subject, "a\x1fb");
    }

    #[test]
    fn create_switch_delete() {
        let dir = repo();
        let p = dir.path();
        create(p, "機能/one", None, true).unwrap();
        let b = list(p).unwrap();
        assert!(b.iter().any(|b| b.name == "機能/one" && b.current));
        assert!(matches!(
            create(p, "機能/one", None, false),
            Err(AppError::BranchExists(_))
        ));

        switch(p, "refs/heads/main").unwrap();
        create(p, "two", Some("refs/heads/機能/one"), false).unwrap();
        assert!(list(p).unwrap().iter().any(|b| b.name == "main" && b.current));

        // An unmerged branch needs force.
        switch(p, "refs/heads/two").unwrap();
        std::fs::write(p.join("b.txt"), "b\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "two"]);
        switch(p, "refs/heads/main").unwrap();
        assert!(matches!(delete(p, "two", false), Err(AppError::BranchNotMerged(_))));
        delete(p, "two", true).unwrap();
        delete(p, "機能/one", false).unwrap();
        assert_eq!(list(p).unwrap().len(), 1);
    }

    #[test]
    fn switch_to_remote_branch_creates_tracking_branch() {
        let origin = repo();
        git(origin.path(), &["branch", "topic"]);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        Git::new(dir.path())
            .args(["clone", "-q", "--"])
            .arg(origin.path().as_os_str())
            .arg(".")
            .run()
            .unwrap();
        switch(p, "refs/remotes/origin/topic").unwrap();
        let b = list(p).unwrap();
        let topic = b.iter().find(|b| b.name == "topic").unwrap();
        assert!(topic.current);
        assert_eq!(topic.upstream.as_deref(), Some("origin/topic"));
        // Again: reuses the local branch.
        switch(p, "refs/heads/main").unwrap();
        switch(p, "refs/remotes/origin/topic").unwrap();
    }

    #[test]
    fn switch_refuses_to_overwrite_local_changes() {
        let dir = repo();
        let p = dir.path();
        create(p, "other", None, true).unwrap();
        std::fs::write(p.join("a.txt"), "other\n").unwrap();
        git(p, &["commit", "-qam", "other"]);
        switch(p, "refs/heads/main").unwrap();
        std::fs::write(p.join("a.txt"), "local\n").unwrap();
        assert!(matches!(
            switch(p, "refs/heads/other"),
            Err(AppError::LocalChangesConflict(_))
        ));
    }

    #[test]
    fn checkout_commit_detaches() {
        let dir = repo();
        let p = dir.path();
        let head = git_out(p, &["rev-parse", "HEAD"]);
        checkout_detached(p, &head).unwrap();
        let st = crate::git::status::get(p).unwrap();
        assert!(st.branch.is_none());
    }

    #[test]
    fn rename_and_upstream() {
        let origin = repo();
        git(origin.path(), &["branch", "topic"]);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        Git::new(p)
            .args(["clone", "-q", "--"])
            .arg(origin.path().as_os_str())
            .arg(".")
            .run()
            .unwrap();
        switch(p, "refs/remotes/origin/topic").unwrap();

        rename(p, "topic", "機能/topic").unwrap();
        let b = list(p).unwrap();
        let renamed = b.iter().find(|b| b.name == "機能/topic").unwrap();
        assert!(renamed.current && !b.iter().any(|b| b.name == "topic"));
        // The upstream moved with the branch.
        assert_eq!(renamed.upstream.as_deref(), Some("origin/topic"));
        assert!(matches!(
            rename(p, "機能/topic", "main"),
            Err(AppError::BranchExists(_))
        ));
        assert!(matches!(rename(p, "nope", "x"), Err(AppError::InvalidInput(_))));

        set_upstream(p, "機能/topic", None).unwrap();
        assert!(
            list(p)
                .unwrap()
                .iter()
                .find(|b| b.name == "機能/topic")
                .unwrap()
                .upstream
                .is_none()
        );
        set_upstream(p, "機能/topic", Some("refs/remotes/origin/main")).unwrap();
        let b = list(p).unwrap();
        assert_eq!(
            b.iter().find(|b| b.name == "機能/topic").unwrap().upstream.as_deref(),
            Some("origin/main")
        );
    }

    #[test]
    fn deletes_a_remote_branch() {
        let origin = repo();
        git(origin.path(), &["branch", "topic"]);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        Git::new(p)
            .args(["clone", "-q", "--"])
            .arg(origin.path().as_os_str())
            .arg(".")
            .run()
            .unwrap();
        delete_remote(p, Vec::new(), "refs/remotes/origin/topic").unwrap();
        assert!(!git_out(origin.path(), &["branch", "--list", "topic"]).contains("topic"));
        assert!(!list(p).unwrap().iter().any(|b| b.name == "origin/topic"));
        // Not a remote branch, or no such remote.
        assert!(matches!(
            delete_remote(p, Vec::new(), "refs/heads/main"),
            Err(AppError::InvalidInput(_))
        ));
        assert!(matches!(
            delete_remote(p, Vec::new(), "refs/remotes/nowhere/x"),
            Err(AppError::InvalidInput(_))
        ));
    }
}
