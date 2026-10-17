use std::path::Path;

use serde::Deserialize;

use super::{Git, PathspecFile, decode, operation, status};
use crate::error::{AppError, Result};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitRequest {
    pub summary: String,
    pub description: String,
    /// Paths to include. For renames both the new and the original path are expected.
    pub paths: Vec<String>,
    /// Replace the last commit instead of creating a new one.
    #[serde(default)]
    pub amend: bool,
}

/// Commits exactly the selected paths, regardless of what is currently staged
/// (like GitHub Desktop, the index is not exposed to the user).
/// Returns the new commit hash.
pub fn commit(repo: &Path, req: &CommitRequest) -> Result<String> {
    let summary = req.summary.trim();
    if summary.is_empty() {
        return Err(AppError::EmptySummary);
    }
    // Amending may just reword the last commit.
    if req.paths.is_empty() && !req.amend {
        return Err(AppError::NothingSelected);
    }
    let mut message = summary.to_string();
    let description = req.description.trim_end();
    if !description.trim().is_empty() {
        message.push_str("\n\n");
        message.push_str(description);
    }
    message.push('\n');

    let in_operation = operation::current(repo)?.is_some();
    if req.amend && in_operation {
        return Err(AppError::GitFailed(
            "cannot amend while a merge, rebase, revert or cherry-pick is in progress".into(),
        ));
    }
    let spec = PathspecFile::new(&req.paths)?;
    if !req.paths.is_empty() {
        Git::new(repo).args(["add", "--all"]).args(spec.args()).run()?;
    }

    let mut git = Git::new(repo).args(["commit", "--file=-"]).stdin(message);
    if req.amend {
        git = git.arg("--amend");
    }
    // A merge (or a stopped revert / cherry-pick) must be concluded as a whole,
    // so commit the full index then.
    if !in_operation {
        git = git.arg("--only");
        if !req.paths.is_empty() {
            git = git.args(spec.args());
        }
    }
    git.run().map_err(AppError::classify_git)?;

    Ok(Git::new(repo)
        .args(["rev-parse", "HEAD"])
        .run_string()?
        .trim()
        .to_string())
}

/// Removes the last commit but keeps its changes, returning its message so it
/// can be edited and committed again. Refused once the commit has been pushed.
pub fn undo_last(repo: &Path) -> Result<String> {
    if operation::current(repo)?.is_some() {
        return Err(AppError::GitFailed(
            "a merge, rebase, revert or cherry-pick is in progress".into(),
        ));
    }
    let st = status::get(repo)?;
    if st.head_oid.is_none() {
        return Err(AppError::NoChanges);
    }
    if st.upstream.is_some() && st.ahead == 0 {
        return Err(AppError::AlreadyPushed);
    }
    let message = decode(&Git::new(repo).args(["log", "-1", "--format=%B"]).run()?.stdout)
        .trim_end()
        .to_string();
    let parent = Git::new(repo)
        .args(["rev-parse", "--verify", "--quiet", "HEAD~1"])
        .ok_codes(&[0, 1])
        .run()?;
    if parent.code == 0 {
        Git::new(repo).args(["reset", "--soft", "HEAD~1"]).run()?;
    } else {
        // The root commit: go back to an unborn branch, keeping the files staged.
        Git::new(repo).args(["update-ref", "-d", "HEAD"]).run()?;
    }
    Ok(message)
}

/// Adds a pattern for `path` (or for every file with its extension) to `.gitignore`.
pub fn ignore(repo: &Path, path: &str, by_extension: bool) -> Result<()> {
    let pattern = if by_extension {
        let name = path.rsplit('/').next().unwrap_or(path);
        match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => format!("*.{}", escape_pattern(ext)),
            _ => return Err(AppError::InvalidInput(path.into())),
        }
    } else {
        format!("/{}", escape_pattern(path))
    };
    let file = repo.join(".gitignore");
    let mut text = match std::fs::read(&file) {
        Ok(bytes) => decode(&bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    if text.lines().any(|l| l.trim_end() == pattern) {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&pattern);
    text.push('\n');
    std::fs::write(&file, text)?;
    Ok(())
}

/// Escapes characters with a special meaning in gitignore patterns.
fn escape_pattern(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "*?[]\\!# ".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::status;
    use crate::testutil::{git, repo as init};

    #[test]
    fn commits_only_selected_files() {
        let dir = init();
        let p = dir.path();
        std::fs::write(p.join("a b.txt"), "a\n").unwrap();
        std::fs::write(p.join("日本語.txt"), "b\n").unwrap();
        std::fs::write(p.join("skip.txt"), "c\n").unwrap();

        let req = CommitRequest {
            summary: "最初のコミット".into(),
            description: "本文".into(),
            paths: vec!["a b.txt".into(), "日本語.txt".into()],
            amend: false,
        };
        let hash = commit(p, &req).unwrap();
        assert_eq!(hash.len(), 40);

        let st = status::get(p).unwrap();
        assert_eq!(st.files.len(), 1);
        assert_eq!(st.files[0].path, "skip.txt");

        let msg = Git::new(p).args(["log", "-1", "--format=%B"]).run_string().unwrap();
        assert_eq!(msg.trim_end(), "最初のコミット\n\n本文");
    }

    #[test]
    fn commits_rename_and_leaves_other_staged_changes() {
        let dir = init();
        let p = dir.path();
        std::fs::write(p.join("old.txt"), "x\n").unwrap();
        std::fs::write(p.join("other.txt"), "y\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "init"]);

        std::fs::rename(p.join("old.txt"), p.join("new.txt")).unwrap();
        std::fs::write(p.join("other.txt"), "z\n").unwrap();
        git(p, &["add", "other.txt"]);

        let req = CommitRequest {
            summary: "rename".into(),
            description: String::new(),
            paths: vec!["new.txt".into(), "old.txt".into()],
            amend: false,
        };
        commit(p, &req).unwrap();

        let st = status::get(p).unwrap();
        assert_eq!(st.files.len(), 1);
        assert_eq!(st.files[0].path, "other.txt");
    }

    #[test]
    fn amend_and_undo() {
        let dir = init();
        let p = dir.path();
        std::fs::write(p.join("a.txt"), "a\n").unwrap();
        commit(
            p,
            &CommitRequest {
                summary: "first".into(),
                description: String::new(),
                paths: vec!["a.txt".into()],
                amend: false,
            },
        )
        .unwrap();
        std::fs::write(p.join("b.txt"), "b\n").unwrap();
        commit(
            p,
            &CommitRequest {
                summary: "second".into(),
                description: "body".into(),
                paths: vec!["b.txt".into()],
                amend: false,
            },
        )
        .unwrap();

        // Reword only.
        commit(
            p,
            &CommitRequest {
                summary: "second!".into(),
                description: String::new(),
                paths: vec![],
                amend: true,
            },
        )
        .unwrap();
        let log = Git::new(p).args(["log", "--format=%s"]).run_string().unwrap();
        assert_eq!(log.lines().collect::<Vec<_>>(), ["second!", "first"]);

        assert_eq!(undo_last(p).unwrap(), "second!");
        let st = status::get(p).unwrap();
        assert_eq!(st.files.len(), 1);
        assert_eq!(st.files[0].path, "b.txt");
        // Undoing the root commit leaves an unborn branch with the files kept.
        assert_eq!(undo_last(p).unwrap(), "first");
        assert!(status::get(p).unwrap().head_oid.is_none());
        assert!(p.join("a.txt").exists() && p.join("b.txt").exists());
    }

    #[test]
    fn ignore_patterns() {
        let dir = init();
        let p = dir.path();
        std::fs::write(p.join(".gitignore"), "target").unwrap();
        ignore(p, "logs/debug [1].log", false).unwrap();
        ignore(p, "logs/debug [1].log", true).unwrap();
        ignore(p, "logs/debug [1].log", true).unwrap();
        let text = std::fs::read_to_string(p.join(".gitignore")).unwrap();
        assert_eq!(text, "target\n/logs/debug\\ \\[1\\].log\n*.log\n");
        assert!(ignore(p, "Makefile", true).is_err());
        assert!(ignore(p, ".env", true).is_err());
    }

    #[test]
    fn rejects_empty_summary() {
        let dir = init();
        let req = CommitRequest {
            summary: "  ".into(),
            description: String::new(),
            paths: vec!["x".into()],
            amend: false,
        };
        assert!(matches!(commit(dir.path(), &req), Err(AppError::EmptySummary)));
    }
}
