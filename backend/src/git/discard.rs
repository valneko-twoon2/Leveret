//! Discarding working tree changes. Files are moved to the Recycle Bin first,
//! so a discard can always be undone from there.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::status::{self, ChangeKind};
use super::{Git, PathspecFile, decode};
use crate::error::{AppError, Result};

/// Discards the changes of `paths` (as listed by status), moving the current
/// files to the Recycle Bin.
pub fn discard(repo: &Path, paths: &[String]) -> Result<()> {
    discard_with(repo, paths, |files| {
        trash::delete_all(files).map_err(|e| AppError::Other(e.to_string()))
    })
}

/// [`discard`] with a custom way of removing files (tests don't use the Recycle Bin).
pub fn discard_with(repo: &Path, paths: &[String], remove: impl FnOnce(&[PathBuf]) -> Result<()>) -> Result<()> {
    if paths.is_empty() {
        return Err(AppError::NothingSelected);
    }
    let wanted: HashSet<&str> = paths.iter().map(String::as_str).collect();
    let status = status::get(repo)?;
    let changes: Vec<_> = status
        .files
        .iter()
        .filter(|f| wanted.contains(f.path.as_str()))
        .collect();
    if changes.iter().any(|f| f.kind == ChangeKind::Conflicted) {
        return Err(AppError::InvalidInput("conflicted files cannot be discarded".into()));
    }

    // Everything currently on disk goes to the Recycle Bin; the rest is restored by git.
    let mut on_disk = Vec::new();
    let mut tracked = Vec::new();
    for f in &changes {
        let full = repo.join(&f.path);
        if f.kind != ChangeKind::Deleted && full.symlink_metadata().is_ok() {
            on_disk.push(full);
        }
        if f.kind != ChangeKind::Untracked {
            tracked.push(f.path.clone());
            tracked.extend(f.orig_path.clone());
        }
    }
    if !on_disk.is_empty() {
        remove(&on_disk)?;
    }
    if tracked.is_empty() {
        return Ok(());
    }

    let (in_head, index_only) = split_by_head(repo, &tracked)?;
    if !index_only.is_empty() {
        // Newly added files: just drop them from the index.
        let mut stdin = Vec::new();
        for p in &index_only {
            stdin.extend_from_slice(p.as_bytes());
            stdin.push(0);
        }
        Git::new(repo)
            .args(["update-index", "--force-remove", "-z", "--stdin"])
            .stdin(stdin)
            .run()?;
    }
    if !in_head.is_empty() {
        let spec = PathspecFile::new(&in_head)?;
        Git::new(repo)
            .args(["restore", "--source=HEAD", "--staged", "--worktree"])
            .args(spec.args())
            .run()?;
    }
    Ok(())
}

/// Splits paths into those that exist in HEAD and those that don't.
fn split_by_head(repo: &Path, paths: &[String]) -> Result<(Vec<String>, Vec<String>)> {
    let mut input = String::new();
    for p in paths {
        input.push_str("HEAD:");
        input.push_str(p);
        input.push('\n');
    }
    let out = Git::new(repo)
        .args(["cat-file", "--batch-check=%(objecttype)"])
        .stdin(input)
        .run()?;
    let text = decode(&out.stdout);
    let (mut in_head, mut index_only) = (Vec::new(), Vec::new());
    for (path, line) in paths.iter().zip(text.lines()) {
        if line.ends_with("missing") || line.ends_with("ambiguous") {
            index_only.push(path.clone());
        } else {
            in_head.push(path.clone());
        }
    }
    Ok((in_head, index_only))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        Git::new(dir).args(args).run().unwrap();
    }

    fn remove(files: &[PathBuf]) -> Result<()> {
        for f in files {
            std::fs::remove_file(f)?;
        }
        Ok(())
    }

    #[test]
    fn discards_every_kind_of_change() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "main"]);
        git(p, &["config", "user.name", "Test"]);
        git(p, &["config", "user.email", "test@example.com"]);
        git(p, &["config", "core.autocrlf", "false"]);
        for f in ["mod.txt", "del.txt", "old.txt", "keep.txt"] {
            std::fs::write(p.join(f), format!("{f}\n")).unwrap();
        }
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "init"]);

        std::fs::write(p.join("mod.txt"), "changed\n").unwrap();
        std::fs::remove_file(p.join("del.txt")).unwrap();
        git(p, &["mv", "old.txt", "new.txt"]);
        std::fs::write(p.join("added.txt"), "added\n").unwrap();
        git(p, &["add", "added.txt"]);
        std::fs::create_dir(p.join("dir")).unwrap();
        std::fs::write(p.join("dir/未追跡.txt"), "u\n").unwrap();
        std::fs::write(p.join("keep.txt"), "keep me\n").unwrap();

        let paths: Vec<String> = ["mod.txt", "del.txt", "new.txt", "added.txt", "dir/未追跡.txt"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        discard_with(p, &paths, remove).unwrap();

        let st = status::get(p).unwrap();
        assert_eq!(st.files.len(), 1, "{:?}", st.files);
        assert_eq!(st.files[0].path, "keep.txt");
        assert_eq!(std::fs::read_to_string(p.join("mod.txt")).unwrap(), "mod.txt\n");
        assert!(p.join("del.txt").exists() && p.join("old.txt").exists());
        assert!(!p.join("new.txt").exists() && !p.join("added.txt").exists());
    }

    #[test]
    fn discards_in_a_repository_without_commits() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q"]);
        std::fs::write(p.join("a.txt"), "a\n").unwrap();
        std::fs::write(p.join("b.txt"), "b\n").unwrap();
        git(p, &["add", "a.txt"]);
        discard_with(p, &["a.txt".into(), "b.txt".into()], remove).unwrap();
        assert!(status::get(p).unwrap().files.is_empty());
    }

    #[test]
    fn rejects_empty_selection() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            discard_with(dir.path(), &[], remove),
            Err(AppError::NothingSelected)
        ));
    }
}
