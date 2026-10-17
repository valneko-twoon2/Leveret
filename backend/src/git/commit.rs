use std::io::Write;
use std::path::Path;

use serde::Deserialize;

use super::{Git, git_path_exists};
use crate::error::{AppError, Result};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitRequest {
    pub summary: String,
    pub description: String,
    /// Paths to include. For renames both the new and the original path are expected.
    pub paths: Vec<String>,
}

/// Commits exactly the selected paths, regardless of what is currently staged
/// (like GitHub Desktop, the index is not exposed to the user).
/// Returns the new commit hash.
pub fn commit(repo: &Path, req: &CommitRequest) -> Result<String> {
    let summary = req.summary.trim();
    if summary.is_empty() {
        return Err(AppError::EmptySummary);
    }
    if req.paths.is_empty() {
        return Err(AppError::NothingSelected);
    }
    let mut message = summary.to_string();
    let description = req.description.trim_end();
    if !description.trim().is_empty() {
        message.push_str("\n\n");
        message.push_str(description);
    }
    message.push('\n');

    // Paths go through a file so long selections don't hit the command line limit.
    let mut pathspec = tempfile::NamedTempFile::new()?;
    for p in &req.paths {
        pathspec.write_all(p.as_bytes())?;
        pathspec.write_all(b"\0")?;
    }
    pathspec.flush()?;
    let pathspec_arg = {
        let mut a = std::ffi::OsString::from("--pathspec-from-file=");
        a.push(pathspec.path());
        a
    };

    Git::new(repo)
        .args(["add", "--all"])
        .arg(&pathspec_arg)
        .arg("--pathspec-file-nul")
        .run()?;

    // A merge must be concluded as a whole, so commit the full index then.
    let mut git = Git::new(repo).args(["commit", "--file=-"]).stdin(message);
    if !git_path_exists(repo, "MERGE_HEAD")? {
        git = git.arg("--only").arg(&pathspec_arg).arg("--pathspec-file-nul");
    }
    git.run().map_err(|e| match e {
        AppError::GitFailed(msg) if msg.contains("user.email") || msg.contains("user.name") => {
            AppError::IdentityMissing(msg)
        }
        e => e,
    })?;

    Ok(Git::new(repo).args(["rev-parse", "HEAD"]).run_string()?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::status;

    fn git(dir: &Path, args: &[&str]) {
        Git::new(dir).args(args).run().unwrap();
    }

    fn init() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q"]);
        git(dir.path(), &["config", "user.name", "Test"]);
        git(dir.path(), &["config", "user.email", "test@example.com"]);
        git(dir.path(), &["config", "core.autocrlf", "false"]);
        dir
    }

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

        let req = CommitRequest { summary: "rename".into(), description: String::new(), paths: vec!["new.txt".into(), "old.txt".into()] };
        commit(p, &req).unwrap();

        let st = status::get(p).unwrap();
        assert_eq!(st.files.len(), 1);
        assert_eq!(st.files[0].path, "other.txt");
    }

    #[test]
    fn rejects_empty_summary() {
        let dir = init();
        let req = CommitRequest { summary: "  ".into(), description: String::new(), paths: vec!["x".into()] };
        assert!(matches!(commit(dir.path(), &req), Err(AppError::EmptySummary)));
    }
}
