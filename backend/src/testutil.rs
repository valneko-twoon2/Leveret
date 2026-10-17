//! Helpers for tests that need real git repositories.
//!
//! Starting git costs tens of milliseconds on Windows, so a configured empty
//! repository is made once per test process and copied for each test instead of
//! running `git init` and `git config` every time. Under `cfg(test)`, `Git` also
//! ignores the user's global and system config (see `git::Git::command`), which
//! keeps tests fast and independent of the machine.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use crate::git::{Git, decode};

/// Template folders of earlier test runs older than this are removed.
const STALE_AFTER: Duration = Duration::from_secs(60 * 60);

/// Runs git in `dir` and panics if it fails.
pub fn git(dir: &Path, args: &[&str]) {
    Git::new(dir).args(args).run().unwrap();
}

/// Runs git in `dir` and returns its trimmed output.
pub fn git_out(dir: &Path, args: &[&str]) -> String {
    decode(&Git::new(dir).args(args).run().unwrap().stdout)
        .trim()
        .to_string()
}

/// An empty file the global and system git config point to in tests.
pub fn empty_config() -> &'static Path {
    static FILE: OnceLock<PathBuf> = OnceLock::new();
    FILE.get_or_init(|| {
        let path = base().join("empty.gitconfig");
        std::fs::write(&path, "").unwrap();
        path
    })
}

/// `%TEMP%\leveret-tests\<pid>`, removing what earlier runs left behind.
fn base() -> &'static Path {
    static BASE: OnceLock<PathBuf> = OnceLock::new();
    BASE.get_or_init(|| {
        let root = std::env::temp_dir().join("leveret-tests");
        if let Ok(entries) = std::fs::read_dir(&root) {
            let now = SystemTime::now();
            for e in entries.flatten() {
                let stale = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .is_ok_and(|t| now.duration_since(t).unwrap_or_default() > STALE_AFTER);
                if stale {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        let dir = root.join(std::process::id().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        dir
    })
}

fn template() -> &'static Path {
    static TEMPLATE: OnceLock<PathBuf> = OnceLock::new();
    TEMPLATE.get_or_init(|| {
        let dir = base().join("template");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // No sample hooks: fewer files to copy.
        git(&dir, &["init", "-q", "-b", "main", "--template="]);
        // Appended directly rather than with `git config` (one process instead of seven).
        let settings = "[user]\n\tname = Test\n\temail = test@example.com\n\
                        [core]\n\tautocrlf = false\n\
                        [gc]\n\tauto = 0\n\
                        [maintenance]\n\tauto = false\n\
                        [commit]\n\tgpgsign = false\n\
                        [tag]\n\tgpgsign = false\n";
        let config = dir.join(".git").join("config");
        let mut text = std::fs::read_to_string(&config).unwrap();
        text.push_str(settings);
        std::fs::write(&config, text).unwrap();
        dir
    })
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A new, empty repository on branch `main` with a test identity.
pub fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&template().join(".git"), &dir.path().join(".git"));
    dir
}

/// A repository with one commit of `a.txt` ("a\n").
pub fn repo_with_commit() -> tempfile::TempDir {
    let dir = repo();
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-qm", "init"]);
    dir
}

/// A new, empty bare repository to use as a remote.
pub fn bare() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "--bare", "-b", "main", "--template="]);
    dir
}
