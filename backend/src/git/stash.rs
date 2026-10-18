//! Stashing local changes away, looking at stashes and bringing them back.

use std::path::Path;

use serde::Serialize;

use super::log::{CommitFile, parse_name_status};
use super::operation::{self, Outcome};
use super::status::ChangeKind;
use super::{Git, PathspecFile, decode, join, status};
use crate::error::{AppError, Result};

const FIELD: char = '\x1f';

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StashEntry {
    /// Position in the stash list (`stash@{index}`).
    pub index: u32,
    /// The stash commit: actions check it, so a shifted list can't hit another stash.
    pub oid: String,
    pub message: String,
    /// Unix seconds.
    pub time: i64,
}

pub fn list(repo: &Path) -> Result<Vec<StashEntry>> {
    let out = Git::new(repo)
        .args(["stash", "list", "--format=%H%x1f%ct%x1f%gs"])
        .run()?;
    Ok(decode(&out.stdout)
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let mut f = line.splitn(3, FIELD);
            let (oid, time, message) = (f.next()?, f.next()?, f.next()?);
            Some(StashEntry {
                index: i as u32,
                oid: oid.to_string(),
                message: message.to_string(),
                time: time.parse().unwrap_or(0),
            })
        })
        .collect())
}

/// Stashes local changes: all of them (including untracked files unless
/// `untracked` is false: the user has hidden them, so they aren't taken away
/// unseen), or only `paths`.
pub fn save(repo: &Path, message: &str, untracked: bool, paths: &[String]) -> Result<()> {
    let st = status::get_with(repo, untracked)?;
    if st.files.is_empty() || (!paths.is_empty() && !st.files.iter().any(|f| paths.contains(&f.path))) {
        return Err(AppError::NoChanges);
    }
    let spec = PathspecFile::new(paths)?;
    let mut git = Git::new(repo).args(["stash", "push"]);
    if paths.is_empty() {
        // Literal pathspecs break `--include-untracked` without paths (untracked
        // files stay behind); no user paths are passed then, so turn them off.
        git = git.envs([("GIT_LITERAL_PATHSPECS".to_string(), "0".to_string())]);
    }
    if untracked {
        git = git.arg("--include-untracked");
    }
    if !message.trim().is_empty() {
        git = git.arg("--message").arg(message.trim());
    }
    if !paths.is_empty() {
        git = git.args(spec.args());
    }
    git.run().map_err(AppError::classify_git)?;
    Ok(())
}

/// `stash@{index}`, after checking it still is the stash `oid` (when given).
fn stash_ref(repo: &Path, index: u32, oid: Option<&str>) -> Result<String> {
    let r = format!("stash@{{{index}}}");
    if let Some(oid) = oid {
        let out = Git::new(repo)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(&r)
            .ok_codes(&[0, 1])
            .run_string()?;
        if !out.trim().starts_with(oid) || oid.is_empty() {
            return Err(AppError::InvalidInput("the stash list has changed; reload it".into()));
        }
    }
    Ok(r)
}

/// Applies a stash and drops it. On conflicts the stash is kept and the
/// conflicted files are left for the user to resolve.
pub fn pop(repo: &Path, index: u32, oid: Option<&str>) -> Result<Outcome> {
    restore(repo, &stash_ref(repo, index, oid)?, "pop")
}

/// Applies a stash and keeps it in the list.
pub fn apply(repo: &Path, index: u32, oid: Option<&str>) -> Result<Outcome> {
    restore(repo, &stash_ref(repo, index, oid)?, "apply")
}

fn restore(repo: &Path, stash: &str, verb: &str) -> Result<Outcome> {
    match Git::new(repo).args(["stash", verb, "--index"]).arg(stash).run() {
        Ok(_) => Ok(Outcome::Done),
        Err(e @ AppError::GitFailed(_)) => {
            if operation::conflicted_paths(repo)?.is_empty() {
                // `--index` fails when the staged state can't be restored; retry without it.
                match Git::new(repo).args(["stash", verb]).arg(stash).run() {
                    Ok(_) => Ok(Outcome::Done),
                    Err(AppError::GitFailed(_)) if !operation::conflicted_paths(repo)?.is_empty() => {
                        Ok(Outcome::Conflicts)
                    }
                    Err(_) => Err(e.classify_git()),
                }
            } else {
                Ok(Outcome::Conflicts)
            }
        }
        Err(e) => Err(e),
    }
}

pub fn drop(repo: &Path, index: u32, oid: Option<&str>) -> Result<()> {
    Git::new(repo)
        .args(["stash", "drop"])
        .arg(stash_ref(repo, index, oid)?)
        .run()?;
    Ok(())
}

/// What a stash holds: tracked changes (the stash commit against `base`) and,
/// when stashed with them, untracked files (the `untracked` commit's tree).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StashDetail {
    pub oid: String,
    /// The commit the changes were stashed on (the stash's first parent).
    pub base: String,
    /// The commit holding the untracked files (third parent), if any.
    pub untracked: Option<String>,
    /// Untracked files have kind `untracked`.
    pub files: Vec<CommitFile>,
}

pub fn detail(repo: &Path, oid: &str) -> Result<StashDetail> {
    let parents = Git::new(repo)
        .args(["rev-list", "--parents", "-n", "1", "--end-of-options", oid])
        .run_string()?;
    let ids: Vec<&str> = parents.split_whitespace().collect();
    let (Some(&base), Some(&full)) = (ids.get(1), ids.first()) else {
        return Err(AppError::InvalidInput(format!("{oid} is not a stash")));
    };
    let untracked = ids.get(3).map(|s| s.to_string());
    let (tracked, extra) = std::thread::scope(|s| {
        let extra = s.spawn(|| match &untracked {
            Some(u) => Git::new(repo)
                .args(["ls-tree", "-r", "-z", "--name-only", u])
                .run()
                .map(Some),
            None => Ok(None),
        });
        let tracked = Git::new(repo)
            .args([
                "diff-tree",
                "-r",
                "-M",
                "--name-status",
                "-z",
                "--no-commit-id",
                base,
                full,
            ])
            .run();
        (tracked, join(extra))
    });
    let mut files = parse_name_status(&decode(&tracked?.stdout));
    if let Some(out) = extra? {
        files.extend(
            decode(&out.stdout)
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(|p| CommitFile {
                    path: p.to_string(),
                    orig_path: None,
                    kind: ChangeKind::Untracked,
                }),
        );
    }
    Ok(StashDetail {
        oid: full.to_string(),
        base: base.to_string(),
        untracked,
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{self, git};

    #[test]
    fn save_list_pop_drop() {
        let dir = testutil::repo_with_commit();
        let p = dir.path();

        assert!(matches!(save(p, "", true, &[]), Err(AppError::NoChanges)));
        std::fs::write(p.join("a.txt"), "changed\n").unwrap();
        std::fs::write(p.join("新規.txt"), "new\n").unwrap();
        save(p, "作業中", true, &[]).unwrap();
        let st = status::get(p).unwrap();
        assert!(st.files.is_empty(), "{:?}", st.files);
        // Hidden untracked files are left alone (and alone aren't something to stash).
        std::fs::write(p.join("keep.txt"), "keep\n").unwrap();
        assert!(matches!(save(p, "", false, &[]), Err(AppError::NoChanges)));
        std::fs::write(p.join("a.txt"), "second\n").unwrap();
        save(p, "", false, &[]).unwrap();
        assert!(p.join("keep.txt").exists());

        let l = list(p).unwrap();
        assert_eq!(l.len(), 2);
        assert!(l[1].message.ends_with("作業中"), "{}", l[1].message);

        assert_eq!(pop(p, 1, Some(&l[1].oid)).unwrap(), Outcome::Done);
        assert_eq!(std::fs::read_to_string(p.join("a.txt")).unwrap(), "changed\n");
        assert!(p.join("新規.txt").exists());

        // The remaining stash touches a.txt too: popping it now conflicts and keeps it.
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "wip"]);
        assert_eq!(pop(p, 0, None).unwrap(), Outcome::Conflicts);
        assert_eq!(list(p).unwrap().len(), 1);
        git(p, &["reset", "-q", "--hard"]);
        // An index that now points at another stash is refused.
        assert!(drop(p, 0, Some(&l[1].oid)).is_err());
        drop(p, 0, Some(&l[0].oid)).unwrap();
        assert!(list(p).unwrap().is_empty());
    }

    #[test]
    fn stash_some_files_look_inside_and_apply() {
        let dir = testutil::repo_with_commit();
        let p = dir.path();
        std::fs::write(p.join("a.txt"), "changed\n").unwrap();
        std::fs::write(p.join("b c.txt"), "untracked [1]\n").unwrap();
        std::fs::write(p.join("keep.txt"), "stays\n").unwrap();
        save(p, "一部", true, &["a.txt".into(), "b c.txt".into()]).unwrap();
        let files: Vec<String> = status::get(p).unwrap().files.into_iter().map(|f| f.path).collect();
        assert_eq!(files, ["keep.txt"]);

        let s = &list(p).unwrap()[0];
        let d = detail(p, &s.oid).unwrap();
        assert_eq!(d.oid, s.oid);
        let mut shown: Vec<(String, ChangeKind)> = d.files.iter().map(|f| (f.path.clone(), f.kind)).collect();
        shown.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            shown,
            [
                ("a.txt".into(), ChangeKind::Modified),
                ("b c.txt".into(), ChangeKind::Untracked)
            ]
        );
        let diff = crate::git::diff::commit_file(p, &d.oid, Some(&d.base), "a.txt", None).unwrap();
        assert_eq!(diff.hunks.len(), 1);

        // Apply keeps the stash; the files come back.
        assert_eq!(apply(p, 0, Some(&s.oid)).unwrap(), Outcome::Done);
        assert_eq!(list(p).unwrap().len(), 1);
        assert_eq!(std::fs::read_to_string(p.join("a.txt")).unwrap(), "changed\n");
        assert!(p.join("b c.txt").exists());
        assert!(matches!(
            save(p, "", true, &["nothing.txt".into()]),
            Err(AppError::NoChanges)
        ));
    }
}
