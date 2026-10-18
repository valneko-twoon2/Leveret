use std::cell::RefCell;

use super::*;
use crate::git::commit::{CommitRequest, commit};
use crate::git::diff::FileDiff;
use crate::git::status;
use crate::testutil::{git, git_out, repo as init};

fn picked(refs: &[(u32, u32)]) -> BTreeSet<LineRef> {
    refs.iter().copied().collect()
}

fn text(patch: Option<Vec<u8>>) -> String {
    String::from_utf8(patch.expect("a patch")).unwrap()
}

const TWO_HUNKS: &str = "diff --git a/f b/f\nindex 1111111..2222222 100644\n--- a/f\n+++ b/f\n\
@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n\
@@ -10,3 +10,4 @@ ctx\n x\n-y\n+Y\n+Z\n z\n";

#[test]
fn stage_keeps_unpicked_deletions_as_context() {
    // Only "+Y" of the second hunk: "-y" stays, "+Z" is left out.
    let p = text(build(TWO_HUNKS.as_bytes(), &picked(&[(1, 2)]), Mode::Stage).unwrap());
    assert_eq!(
        p,
        "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -10,3 +10,4 @@\n x\n y\n+Y\n z\n"
    );
}

#[test]
fn stage_shifts_later_hunks_by_earlier_ones() {
    let p = text(
        build(
            TWO_HUNKS.as_bytes(),
            &picked(&[(0, 2), (1, 1), (1, 2), (1, 3)]),
            Mode::Stage,
        )
        .unwrap(),
    );
    // First hunk: only "+B" (so the old line stays): 3 lines become 4, which moves
    // the second hunk down by one in the new file.
    assert!(p.contains("@@ -1,3 +1,4 @@\n a\n b\n+B\n c\n"), "{p}");
    assert!(p.contains("@@ -10,3 +11,4 @@\n x\n-y\n+Y\n+Z\n z\n"), "{p}");
}

#[test]
fn discard_keeps_unpicked_additions_as_context() {
    // Undo only "-y": the patch's new side must equal the working tree.
    let p = text(build(TWO_HUNKS.as_bytes(), &picked(&[(1, 1)]), Mode::Discard).unwrap());
    assert!(p.ends_with("@@ -10,5 +10,4 @@\n x\n-y\n Y\n Z\n z\n"), "{p}");
}

#[test]
fn nothing_picked_is_no_patch_and_bad_lines_are_refused() {
    assert!(
        build(TWO_HUNKS.as_bytes(), &picked(&[(0, 0)]), Mode::Stage)
            .unwrap()
            .is_none()
    );
    assert!(build(TWO_HUNKS.as_bytes(), &picked(&[(2, 0)]), Mode::Stage).is_err());
    assert!(build(TWO_HUNKS.as_bytes(), &picked(&[(0, 9)]), Mode::Discard).is_err());
}

#[test]
fn end_of_file_newline_changes_stay_whole() {
    // The old file ended without a newline; picking just "+c" must take "-b"/"+b" too.
    let raw = "--- a/f\n+++ b/f\n@@ -1,2 +1,3 @@\n a\n-b\n\\ No newline at end of file\n+b\n+c\n";
    let p = text(build(raw.as_bytes(), &picked(&[(0, 4)]), Mode::Stage).unwrap());
    assert!(
        p.ends_with("@@ -1,2 +1,3 @@\n a\n-b\n\\ No newline at end of file\n+b\n+c\n"),
        "{p}"
    );
    // A marker after a dropped addition goes with it.
    let raw = "--- a/f\n+++ b/f\n@@ -1 +1,3 @@\n a\n+b\n+c\n\\ No newline at end of file\n";
    let p = text(build(raw.as_bytes(), &picked(&[(0, 1)]), Mode::Stage).unwrap());
    assert!(p.ends_with("@@ -1,1 +1,2 @@\n a\n+b\n"), "{p}");
}

#[test]
fn keeps_carriage_returns_and_new_file_headers() {
    let raw = b"diff --git a/n b/n\nnew file mode 100644\nindex 0000000..1234567\n--- /dev/null\n+++ b/n\n@@ -0,0 +1,2 @@\n+a\r\n+b\r\n";
    let p = text(build(raw, &picked(&[(0, 1)]), Mode::Stage).unwrap());
    assert_eq!(
        p,
        "diff --git a/n b/n\nnew file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1,1 @@\n+b\r\n"
    );
    // Discarding works on the file as it is: a modification of it.
    let p = text(build(raw, &picked(&[(0, 0)]), Mode::Discard).unwrap());
    assert_eq!(p, "diff --git a/n b/n\n--- a/n\n+++ b/n\n@@ -1,1 +1,2 @@\n+a\r\n b\r\n");
}

#[test]
fn modification_header_handles_quoted_and_spaced_names() {
    let raw = diff::split(b"diff --git \"a/\\tx\" \"b/\\tx\"\nnew file mode 100644\n--- /dev/null\n+++ \"b/\\tx\"\n");
    assert_eq!(
        modification_header(&raw),
        b"diff --git \"a/\\tx\" \"b/\\tx\"\n--- \"a/\\tx\"\n+++ \"b/\\tx\"\n"
    );
    let raw = diff::split(b"diff --git a/a b b/a b\n--- a/a b\t\n+++ b/a b\t\n");
    assert_eq!(
        modification_header(&raw),
        b"diff --git a/a b b/a b\n--- a/a b\t\n+++ b/a b\t\n"
    );
}

// --- against repositories ------------------------------------------------------

/// `[hunk, line]` of the added ("+x") or deleted ("-x") lines with these texts.
fn lines_of(d: &FileDiff, wanted: &[&str]) -> Vec<[u32; 2]> {
    let mut out = Vec::new();
    for (h, hunk) in d.hunks.iter().enumerate() {
        for (l, line) in hunk.lines.iter().enumerate() {
            let sign = match line.kind {
                LineKind::Add => '+',
                LineKind::Delete => '-',
                _ => continue,
            };
            if wanted.contains(&format!("{sign}{}", line.text).as_str()) {
                out.push([h as u32, l as u32]);
            }
        }
    }
    assert_eq!(out.len(), wanted.len(), "lines {wanted:?} in {d:?}");
    out
}

fn selection(repo: &Path, path: &str, untracked: bool, wanted: &[&str]) -> LineSelection {
    let d = diff::working(repo, path, None, untracked).unwrap();
    assert!(d.selectable, "{path} should be selectable");
    LineSelection {
        path: path.into(),
        untracked,
        lines: lines_of(&d, wanted),
        token: d.token,
    }
}

fn commit_lines(repo: &Path, paths: &[&str], partial: Vec<LineSelection>, amend: bool) -> Result<String> {
    commit(
        repo,
        &CommitRequest {
            summary: "部分コミット".into(),
            description: String::new(),
            paths: paths.iter().map(|p| p.to_string()).collect(),
            amend,
            partial,
        },
    )
}

fn numbered(lines: &[&str]) -> String {
    lines.iter().map(|l| format!("{l}\n")).collect()
}

/// 20 lines committed, so changes at lines 2 and 15 make two hunks.
fn repo_with_file(name: &str) -> (tempfile::TempDir, Vec<String>) {
    let dir = init();
    let lines: Vec<String> = (1..=20).map(|i| format!("line {i}")).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    std::fs::write(dir.path().join(name), numbered(&refs)).unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-qm", "base"]);
    (dir, lines)
}

#[test]
fn commits_picked_lines_and_keeps_the_rest_as_changes() {
    let name = "日本語 file.txt";
    let (dir, mut lines) = repo_with_file(name);
    let p = dir.path();
    lines[1] = "changed 2".into();
    lines[14] = "changed 15".into();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    std::fs::write(p.join(name), numbered(&refs)).unwrap();
    std::fs::write(p.join("other.txt"), "whole\n").unwrap();

    let sel = selection(p, name, false, &["-line 2", "+changed 2"]);
    commit_lines(p, &["other.txt"], vec![sel], false).unwrap();

    let committed = git_out(p, &["show", &format!("HEAD:{name}")]);
    assert!(committed.contains("changed 2") && committed.contains("line 15") && !committed.contains("changed 15"));
    assert_eq!(git_out(p, &["show", "HEAD:other.txt"]), "whole");
    // The rest is still a change in the working tree, and nothing is left staged.
    assert!(std::fs::read_to_string(p.join(name)).unwrap().contains("changed 15"));
    let st = status::get(p).unwrap();
    assert_eq!(st.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), [name]);
    assert_eq!(git_out(p, &["diff", "--cached", "--name-only"]), "");
}

#[test]
fn commits_part_of_an_untracked_file_and_amends() {
    let dir = init();
    let p = dir.path();
    std::fs::write(p.join("new.txt"), "one\ntwo\nthree").unwrap();
    let sel = selection(p, "new.txt", true, &["+one", "+three"]);
    commit_lines(p, &[], vec![sel], false).unwrap();
    // The last line had no newline and stays that way.
    assert_eq!(git_out(p, &["cat-file", "-p", "HEAD:new.txt"]), "one\nthree");
    assert_eq!(status::get(p).unwrap().files[0].kind, status::ChangeKind::Modified);

    // Amend with the remaining line.
    let sel = selection(p, "new.txt", false, &["+two"]);
    commit_lines(p, &[], vec![sel], true).unwrap();
    assert_eq!(git_out(p, &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(git_out(p, &["cat-file", "-p", "HEAD:new.txt"]), "one\ntwo\nthree");
    assert!(status::get(p).unwrap().files.is_empty());
}

#[test]
fn refuses_lines_picked_in_an_older_diff() {
    let (dir, _) = repo_with_file("f.txt");
    let p = dir.path();
    std::fs::write(p.join("f.txt"), "new\n").unwrap();
    let sel = selection(p, "f.txt", false, &["+new"]);
    std::fs::write(p.join("f.txt"), "newer\n").unwrap();
    let err = commit_lines(p, &[], vec![sel.clone()], false).unwrap_err();
    assert!(matches!(err, AppError::DiffChanged(_)), "{err:?}");
    assert!(matches!(
        discard_with(p, &sel, |_| Ok(())),
        Err(AppError::DiffChanged(_))
    ));
}

#[test]
fn discards_picked_lines_after_backing_up_the_file() {
    let (dir, mut lines) = repo_with_file("f.txt");
    let p = dir.path();
    lines[1] = "changed 2".into();
    lines[14] = "changed 15".into();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    std::fs::write(p.join("f.txt"), numbered(&refs)).unwrap();

    let backups = RefCell::new(Vec::new());
    let sel = selection(p, "f.txt", false, &["-line 15", "+changed 15"]);
    discard_with(p, &sel, |files| {
        backups.borrow_mut().push(std::fs::read_to_string(&files[0]).unwrap());
        Ok(())
    })
    .unwrap();
    let now = std::fs::read_to_string(p.join("f.txt")).unwrap();
    assert!(now.contains("changed 2") && now.contains("line 15") && !now.contains("changed 15"));
    assert!(backups.borrow()[0].contains("changed 15"));

    // Untracked: the picked lines are removed from the file.
    std::fs::write(p.join("u.txt"), "keep\ndrop\nkeep too\n").unwrap();
    let sel = selection(p, "u.txt", true, &["+drop"]);
    discard_with(p, &sel, |_| Ok(())).unwrap();
    assert_eq!(std::fs::read_to_string(p.join("u.txt")).unwrap(), "keep\nkeep too\n");
}

#[test]
fn line_endings_follow_autocrlf() {
    let dir = init();
    let p = dir.path();
    git(p, &["config", "core.autocrlf", "true"]);
    std::fs::write(p.join("w.txt"), "a\r\nb\r\nc\r\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-qm", "base"]);
    std::fs::write(p.join("w.txt"), "a\r\nB\r\nc\r\nd\r\n").unwrap();
    std::fs::write(p.join("n.txt"), "x\r\ny\r\n").unwrap();

    let sel = selection(p, "w.txt", false, &["-b", "+B"]);
    let new = selection(p, "n.txt", true, &["+y"]);
    commit_lines(p, &[], vec![sel, new], false).unwrap();
    // Stored with LF like `git add` would, the working tree keeps CRLF.
    assert_eq!(git_out(p, &["cat-file", "-p", "HEAD:w.txt"]), "a\nB\nc");
    assert_eq!(git_out(p, &["cat-file", "-p", "HEAD:n.txt"]), "y");
    assert_eq!(std::fs::read(p.join("w.txt")).unwrap(), b"a\r\nB\r\nc\r\nd\r\n");

    let sel = selection(p, "w.txt", false, &["+d"]);
    discard_with(p, &sel, |_| Ok(())).unwrap();
    assert_eq!(std::fs::read(p.join("w.txt")).unwrap(), b"a\r\nB\r\nc\r\n");
}

#[test]
fn works_before_the_first_commit_and_for_hunks_picked_by_number() {
    let dir = init();
    let p = dir.path();
    std::fs::write(p.join("a.txt"), "1\n2\n").unwrap();
    git(p, &["add", "a.txt"]);
    let sel = select_hunks(p, "a.txt", false, &[0]).unwrap();
    assert_eq!(sel.lines.len(), 2);
    commit_lines(p, &[], vec![sel], false).unwrap();
    assert_eq!(git_out(p, &["cat-file", "-p", "HEAD:a.txt"]), "1\n2");
    assert!(select_hunks(p, "a.txt", false, &[0]).is_err(), "no changes left");
}

#[test]
fn renamed_and_deleted_files_are_whole_file_only() {
    let (dir, _) = repo_with_file("f.txt");
    let p = dir.path();
    std::fs::remove_file(p.join("f.txt")).unwrap();
    assert!(!diff::working(p, "f.txt", None, false).unwrap().selectable);
    git(p, &["checkout", "--", "f.txt"]);
    git(p, &["mv", "f.txt", "g.txt"]);
    assert!(!diff::working(p, "g.txt", Some("f.txt"), false).unwrap().selectable);
    assert!(!diff::working_with(p, "g.txt", None, false, true).unwrap().selectable);
}
