//! Stashing local changes away and bringing them back.

use std::path::Path;

use serde::Serialize;

use super::operation::{self, Outcome};
use super::{Git, decode, status};
use crate::error::{AppError, Result};

const FIELD: char = '\x1f';

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StashEntry {
    /// Position in the stash list (`stash@{index}`).
    pub index: u32,
    pub message: String,
    /// Unix seconds.
    pub time: i64,
}

pub fn list(repo: &Path) -> Result<Vec<StashEntry>> {
    let out = Git::new(repo).args(["stash", "list", "--format=%ct%x1f%gs"]).run()?;
    Ok(decode(&out.stdout)
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let (time, message) = line.split_once(FIELD)?;
            Some(StashEntry {
                index: i as u32,
                message: message.to_string(),
                time: time.parse().unwrap_or(0),
            })
        })
        .collect())
}

/// Stashes all local changes, including untracked files.
pub fn save(repo: &Path, message: &str) -> Result<()> {
    if status::get(repo)?.files.is_empty() {
        return Err(AppError::NoChanges);
    }
    // Literal pathspecs break `--include-untracked` (untracked files stay behind);
    // no user paths are passed here, so it is safe to turn them off.
    let mut git = Git::new(repo)
        .envs([("GIT_LITERAL_PATHSPECS".to_string(), "0".to_string())])
        .args(["stash", "push", "--include-untracked"]);
    if !message.trim().is_empty() {
        git = git.arg("--message").arg(message.trim());
    }
    git.run().map_err(AppError::classify_git)?;
    Ok(())
}

fn stash_ref(index: u32) -> String {
    format!("stash@{{{index}}}")
}

/// Applies a stash and drops it. On conflicts the stash is kept and the
/// conflicted files are left for the user to resolve.
pub fn pop(repo: &Path, index: u32) -> Result<Outcome> {
    match Git::new(repo)
        .args(["stash", "pop", "--index"])
        .arg(stash_ref(index))
        .run()
    {
        Ok(_) => Ok(Outcome::Done),
        Err(e @ AppError::GitFailed(_)) => {
            if operation::conflicted_paths(repo)?.is_empty() {
                // `--index` fails when the staged state can't be restored; retry without it.
                match Git::new(repo).args(["stash", "pop"]).arg(stash_ref(index)).run() {
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

pub fn drop(repo: &Path, index: u32) -> Result<()> {
    Git::new(repo).args(["stash", "drop"]).arg(stash_ref(index)).run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        Git::new(dir).args(args).run().unwrap();
    }

    #[test]
    fn save_list_pop_drop() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"]);
        git(p, &["config", "user.name", "Test"]);
        git(p, &["config", "user.email", "test@example.com"]);
        git(p, &["config", "core.autocrlf", "false"]);
        std::fs::write(p.join("a.txt"), "a\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "init"]);

        assert!(matches!(save(p, ""), Err(AppError::NoChanges)));
        std::fs::write(p.join("a.txt"), "changed\n").unwrap();
        std::fs::write(p.join("新規.txt"), "new\n").unwrap();
        save(p, "作業中").unwrap();
        let st = status::get(p).unwrap();
        assert!(st.files.is_empty(), "{:?}", st.files);
        std::fs::write(p.join("a.txt"), "second\n").unwrap();
        save(p, "").unwrap();

        let l = list(p).unwrap();
        assert_eq!(l.len(), 2);
        assert!(l[1].message.ends_with("作業中"), "{}", l[1].message);

        assert_eq!(pop(p, 1).unwrap(), Outcome::Done);
        assert_eq!(std::fs::read_to_string(p.join("a.txt")).unwrap(), "changed\n");
        assert!(p.join("新規.txt").exists());

        // The remaining stash touches a.txt too: popping it now conflicts and keeps it.
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "wip"]);
        assert_eq!(pop(p, 0).unwrap(), Outcome::Conflicts);
        assert_eq!(list(p).unwrap().len(), 1);
        git(p, &["reset", "-q", "--hard"]);
        drop(p, 0).unwrap();
        assert!(list(p).unwrap().is_empty());
    }
}
