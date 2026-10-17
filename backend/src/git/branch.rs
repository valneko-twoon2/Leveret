//! Listing, creating, switching and deleting branches.

use std::path::Path;

use serde::Serialize;

use super::{Git, decode};
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
        let local = remote.split_once('/').map(|(_, b)| b).filter(|b| !b.is_empty());
        let local = local.ok_or_else(|| AppError::InvalidInput(full_ref.into()))?;
        if branch_exists(repo, local)? {
            Git::new(repo).args(["switch", "--no-guess", local])
        } else {
            Git::new(repo).args(["switch", "-c", local, "--track", remote])
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

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        Git::new(dir).args(args).run().unwrap();
    }

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"]);
        git(p, &["config", "user.name", "Test"]);
        git(p, &["config", "user.email", "test@example.com"]);
        std::fs::write(p.join("a.txt"), "a\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "init"]);
        dir
    }

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
        let head = Git::new(p).args(["rev-parse", "HEAD"]).run_string().unwrap();
        checkout_detached(p, head.trim()).unwrap();
        let st = crate::git::status::get(p).unwrap();
        assert!(st.branch.is_none());
    }
}
