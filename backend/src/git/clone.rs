use std::path::Path;

use super::{Git, Phases, Progress, progress_reporter};
use crate::error::{AppError, Result};

const PHASES: &Phases = &[
    ("Counting objects", 0, 5),
    ("Compressing objects", 5, 10),
    ("Receiving objects", 10, 80),
    ("Resolving deltas", 80, 95),
    ("Updating files", 95, 100),
];

/// Clones `url` into `dest` (which must not exist or be an empty folder).
pub fn clone(url: &str, dest: &Path, envs: Vec<(String, String)>, on_progress: impl FnMut(Progress)) -> Result<()> {
    if dest.exists() && std::fs::read_dir(dest)?.next().is_some() {
        return Err(AppError::CloneTargetExists(dest.display().to_string()));
    }
    let parent = dest
        .parent()
        .ok_or_else(|| AppError::InvalidInput(dest.display().to_string()))?;
    std::fs::create_dir_all(parent)?;

    let result = Git::new(parent)
        .envs(envs)
        .args(["clone", "--progress", "--", url])
        .arg(dest.as_os_str())
        .run_with_progress(progress_reporter(PHASES, on_progress));

    if result.is_err() && dest.exists() {
        // Don't leave a half-cloned folder behind.
        let _ = std::fs::remove_dir_all(dest);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::parse_progress;

    #[test]
    fn parses_progress_lines() {
        let p = parse_progress("Receiving objects:  50% (500/1000), 1.20 MiB | 1.00 MiB/s", PHASES).unwrap();
        assert_eq!(p.phase, "Receiving objects");
        assert_eq!(p.percent, 45);
        let p = parse_progress("remote: Counting objects: 100% (10/10), done.", PHASES).unwrap();
        assert_eq!(p.percent, 5);
        assert_eq!(
            parse_progress("Resolving deltas: 100% (3/3), done.", PHASES)
                .unwrap()
                .percent,
            95
        );
        assert!(parse_progress("Cloning into 'x'...", PHASES).is_none());
    }

    #[test]
    fn clones_a_local_repository() {
        let src = tempfile::tempdir().unwrap();
        let s = src.path();
        let git = |args: &[&str]| Git::new(s).args(args).run().unwrap();
        git(&["init", "-q"]);
        git(&["config", "user.name", "T"]);
        git(&["config", "user.email", "t@example.com"]);
        std::fs::write(s.join("a.txt"), "a\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "init"]);

        let dst = tempfile::tempdir().unwrap();
        let dest = dst.path().join("copy");
        clone(&s.display().to_string(), &dest, Vec::new(), |_| {}).unwrap();
        assert!(dest.join("a.txt").exists());

        // A non-empty target is refused.
        let err = clone(&s.display().to_string(), &dest, Vec::new(), |_| {}).unwrap_err();
        assert!(matches!(err, AppError::CloneTargetExists(_)));
    }

    #[test]
    fn failed_clone_cleans_up() {
        let dst = tempfile::tempdir().unwrap();
        let dest = dst.path().join("missing");
        let missing = dst.path().join("does-not-exist").display().to_string();
        assert!(clone(&missing, &dest, Vec::new(), |_| {}).is_err());
        assert!(!dest.exists());
    }
}
