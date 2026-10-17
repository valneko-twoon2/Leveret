//! Helpers for resolving conflicts: finding conflict markers left in files, and
//! taking one side's version of a file.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{Git, PathspecFile, decode};
use crate::error::{AppError, Result};

/// Files larger than this are not scanned for markers.
const MAX_SCAN_BYTES: u64 = 8 * 1024 * 1024;

/// The files among `paths` that still contain conflict markers
/// (`<<<<<<< ` and `>>>>>>> ` at the start of a line).
pub fn files_with_markers(repo: &Path, paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter(|p| {
            let file = repo.join(p);
            let small = file.metadata().is_ok_and(|m| m.is_file() && m.len() <= MAX_SCAN_BYTES);
            small && std::fs::read(&file).is_ok_and(|bytes| has_markers(&bytes))
        })
        .cloned()
        .collect()
}

fn has_markers(bytes: &[u8]) -> bool {
    // Binary files can't hold text conflict markers.
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return false;
    }
    let mut start = false;
    for line in bytes.split(|&b| b == b'\n') {
        if line.starts_with(b"<<<<<<< ") || line == b"<<<<<<<" || line == b"<<<<<<<\r" {
            start = true;
        } else if start && (line.starts_with(b">>>>>>> ") || line == b">>>>>>>" || line == b">>>>>>>\r") {
            return true;
        }
    }
    false
}

/// Which version of a conflicted file to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    /// HEAD: the current branch (during a rebase, the branch being rebased onto).
    Ours,
    /// The commit being merged, cherry-picked, reverted or replayed.
    Theirs,
}

/// Resolves conflicted files by taking one side's version and marking them as
/// resolved. The current files (with conflict markers and any manual edits) are
/// moved to the Recycle Bin first. A file that side deleted is deleted.
pub fn take_side(repo: &Path, paths: &[String], side: Side) -> Result<()> {
    take_side_with(repo, paths, side, |files| {
        trash::delete_all(files).map_err(|e| AppError::Other(e.to_string()))
    })
}

/// [`take_side`] with a custom way of removing files (tests don't use the Recycle Bin).
pub fn take_side_with(
    repo: &Path,
    paths: &[String],
    side: Side,
    remove: impl FnOnce(&[PathBuf]) -> Result<()>,
) -> Result<()> {
    if paths.is_empty() {
        return Err(AppError::NothingSelected);
    }
    let stage = match side {
        Side::Ours => "2",
        Side::Theirs => "3",
    };
    // Which of the paths are conflicted, and which have a version on the chosen side
    // (ls-files has no --pathspec-from-file, so filter here).
    let unmerged = Git::new(repo).args(["ls-files", "-u", "-z"]).run()?;
    let mut conflicted = Vec::new();
    let mut with_side = Vec::new();
    for entry in decode(&unmerged.stdout).split('\0').filter(|e| !e.is_empty()) {
        // <mode> <object> <stage>\t<path>
        let Some((meta, path)) = entry.split_once('\t') else {
            continue;
        };
        if !paths.iter().any(|p| p == path) {
            continue;
        }
        if !conflicted.iter().any(|p: &String| p == path) {
            conflicted.push(path.to_string());
        }
        if meta.split(' ').nth(2) == Some(stage) {
            with_side.push(path.to_string());
        }
    }
    if let Some(p) = paths.iter().find(|p| !conflicted.contains(p)) {
        return Err(AppError::InvalidInput(format!("'{p}' has no conflict")));
    }

    let on_disk: Vec<PathBuf> = conflicted
        .iter()
        .map(|p| repo.join(p))
        .filter(|f| f.symlink_metadata().is_ok())
        .collect();
    if !on_disk.is_empty() {
        remove(&on_disk)?;
    }
    if !with_side.is_empty() {
        let spec = PathspecFile::new(&with_side)?;
        let flag = if side == Side::Ours { "--ours" } else { "--theirs" };
        Git::new(repo).args(["checkout", flag]).args(spec.args()).run()?;
    }
    // Stage the result: taken versions are added, files that side deleted are removed.
    let spec = PathspecFile::new(&conflicted)?;
    Git::new(repo).args(["add", "--all"]).args(spec.args()).run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, repo};

    #[test]
    fn finds_markers() {
        assert!(has_markers(b"a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> branch\nb\n"));
        assert!(has_markers(b"<<<<<<< HEAD\r\nx\r\n=======\r\ny\r\n>>>>>>> b\r\n"));
        assert!(!has_markers(b"no markers\n=======\nheading\n"));
        assert!(!has_markers(b">>>>>>> only the end\n<<<<<<< only the start\n"));
        assert!(!has_markers(b"\0<<<<<<< HEAD\n>>>>>>> b\n"));
        assert!(!has_markers(b"  <<<<<<< indented\n  >>>>>>> indented\n"));
    }

    fn conflict() -> tempfile::TempDir {
        let dir = repo();
        let p = dir.path();
        std::fs::write(p.join("a.txt"), "base\n").unwrap();
        std::fs::write(p.join("gone.txt"), "base\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "base"]);
        git(p, &["switch", "-qc", "other"]);
        std::fs::write(p.join("a.txt"), "theirs\n").unwrap();
        git(p, &["rm", "-q", "gone.txt"]);
        git(p, &["commit", "-qam", "theirs"]);
        git(p, &["switch", "-q", "main"]);
        std::fs::write(p.join("a.txt"), "ours\n").unwrap();
        std::fs::write(p.join("gone.txt"), "ours\n").unwrap();
        git(p, &["commit", "-qam", "ours"]);
        Git::new(p)
            .args(["merge", "-q", "other"])
            .ok_codes(&[0, 1])
            .run()
            .unwrap();
        dir
    }

    #[test]
    fn takes_one_side_and_keeps_old_files_out_of_the_way() {
        let dir = conflict();
        let p = dir.path();
        let all = vec!["a.txt".to_string(), "gone.txt".to_string()];
        assert_eq!(files_with_markers(p, &all), ["a.txt"]);

        let mut removed = Vec::new();
        take_side_with(p, &all, Side::Theirs, |files| {
            for f in files {
                removed.push(f.file_name().unwrap().to_string_lossy().into_owned());
                std::fs::remove_file(f)?;
            }
            Ok(())
        })
        .unwrap();
        removed.sort();
        assert_eq!(removed, ["a.txt", "gone.txt"]);
        assert_eq!(std::fs::read_to_string(p.join("a.txt")).unwrap(), "theirs\n");
        // Their side deleted it.
        assert!(!p.join("gone.txt").exists());
        assert!(super::super::operation::conflicted_paths(p).unwrap().is_empty());
    }

    #[test]
    fn takes_our_side_and_rejects_files_without_conflicts() {
        let dir = conflict();
        let p = dir.path();
        let keep = |_: &[PathBuf]| Ok(());
        assert!(take_side_with(p, &["nope.txt".to_string()], Side::Ours, keep).is_err());
        take_side_with(p, &["a.txt".to_string(), "gone.txt".to_string()], Side::Ours, |files| {
            for f in files {
                std::fs::remove_file(f)?;
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(std::fs::read_to_string(p.join("a.txt")).unwrap(), "ours\n");
        assert_eq!(std::fs::read_to_string(p.join("gone.txt")).unwrap(), "ours\n");
    }
}
