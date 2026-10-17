//! End-to-end checks of the git wrappers against a real temporary repository.

use std::path::Path;

use super::Git;
use super::diff::{self, LineKind};
use super::log::{self, LogScope, RefKind};
use super::status::{self, ChangeKind};

fn git(dir: &Path, args: &[&str]) {
    Git::new(dir).args(args).run().unwrap();
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"]);
    git(p, &["config", "user.name", "Test"]);
    git(p, &["config", "user.email", "test@example.com"]);
    git(p, &["config", "core.autocrlf", "false"]);
    dir
}

#[test]
fn empty_repository() {
    let dir = repo();
    assert!(log::get(dir.path(), LogScope::Head, 100).unwrap().is_empty());
    let st = status::get(dir.path()).unwrap();
    assert_eq!(st.branch.as_deref(), Some("main"));
    assert!(st.head_oid.is_none());
    // Without HEAD, a staged file is diffed against the empty tree.
    std::fs::write(dir.path().join("a.txt"), "one\n").unwrap();
    git(dir.path(), &["add", "a.txt"]);
    let d = diff::working(dir.path(), "a.txt", None, false).unwrap();
    assert_eq!(d.hunks[0].lines[0].kind, LineKind::Add);
}

#[test]
fn history_with_merge_details_and_diffs() {
    let dir = repo();
    let p = dir.path();
    std::fs::write(p.join("a.txt"), "one\ntwo\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-qm", "初期コミット"]);
    git(p, &["switch", "-qc", "feature"]);
    std::fs::write(p.join("日本語.txt"), "こんにちは\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-qm", "feature work"]);
    git(p, &["switch", "-q", "main"]);
    std::fs::write(p.join("a.txt"), "one\nTWO\n").unwrap();
    git(p, &["commit", "-qam", "main work"]);
    git(p, &["merge", "-q", "--no-ff", "feature", "-m", "Merge feature"]);
    git(p, &["tag", "v1"]);

    let entries = log::get(p, LogScope::Head, 100).unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].subject, "Merge feature");
    assert_eq!(entries[0].parents.len(), 2);
    assert!(
        entries[0]
            .refs
            .iter()
            .any(|r| r.kind == RefKind::Branch && r.current && r.name == "main")
    );
    assert!(entries[0].refs.iter().any(|r| r.kind == RefKind::Tag && r.name == "v1"));
    assert_eq!(entries[3].subject, "初期コミット");
    assert!(
        entries.iter().any(|e| e.graph.col == 1),
        "side branch should get its own lane"
    );

    let limited = log::get(p, LogScope::All, 2).unwrap();
    assert_eq!(limited.len(), 2);

    // Commit detail of the feature commit and its diff.
    let feature = entries.iter().find(|e| e.subject == "feature work").unwrap();
    let detail = log::detail(p, &feature.hash).unwrap();
    assert_eq!(detail.message, "feature work");
    assert_eq!(detail.files.len(), 1);
    assert_eq!(detail.files[0].path, "日本語.txt");
    assert_eq!(detail.files[0].kind, ChangeKind::Added);
    let d = diff::commit_file(
        p,
        &detail.hash,
        detail.parents.first().map(String::as_str),
        "日本語.txt",
        None,
    )
    .unwrap();
    assert_eq!(d.hunks[0].lines[0].text, "こんにちは");

    // Root commit diff (against the empty tree).
    let root = log::detail(p, &entries[3].hash).unwrap();
    assert!(root.parents.is_empty());
    let d = diff::commit_file(p, &root.hash, None, "a.txt", None).unwrap();
    assert_eq!(d.hunks[0].lines.len(), 2);

    // Working tree diff and untracked file preview.
    std::fs::write(p.join("a.txt"), "one\nthree\n").unwrap();
    std::fs::write(p.join("new.txt"), "x\ny\n").unwrap();
    let st = status::get(p).unwrap();
    assert_eq!(st.files.len(), 2);
    let d = diff::working(p, "a.txt", None, false).unwrap();
    let kinds: Vec<LineKind> = d.hunks[0].lines.iter().map(|l| l.kind).collect();
    assert!(kinds.contains(&LineKind::Add) && kinds.contains(&LineKind::Delete));
    let d = diff::working(p, "new.txt", None, true).unwrap();
    assert_eq!(d.hunks[0].lines.len(), 2);
}

#[test]
fn status_leaves_out_ignored_and_optionally_untracked_files() {
    let dir = repo();
    let p = dir.path();
    std::fs::write(p.join(".gitignore"), "target/\n*.log\n").unwrap();
    std::fs::write(p.join("a.txt"), "one\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-qm", "init"]);
    std::fs::create_dir(p.join("target")).unwrap();
    std::fs::write(p.join("target/out.bin"), "x").unwrap();
    std::fs::write(p.join("debug.log"), "x").unwrap();
    std::fs::write(p.join("new.txt"), "x").unwrap();
    std::fs::write(p.join("a.txt"), "two\n").unwrap();

    let paths = |untracked| -> Vec<(String, ChangeKind)> {
        status::get_with(p, untracked)
            .unwrap()
            .files
            .into_iter()
            .map(|f| (f.path, f.kind))
            .collect()
    };
    assert_eq!(
        paths(true),
        [("a.txt".to_string(), ChangeKind::Modified), ("new.txt".to_string(), ChangeKind::Untracked)]
    );
    assert_eq!(paths(false), [("a.txt".to_string(), ChangeKind::Modified)]);
}
