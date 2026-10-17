use std::path::Path;

use serde::Serialize;

use super::{Git, decode, empty_tree, has_head};
use crate::error::Result;

/// Larger diffs are cut off to keep the UI responsive.
const MAX_LINES: usize = 20_000;
/// Untracked files larger than this are not loaded.
const MAX_UNTRACKED_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    Context,
    Add,
    Delete,
    /// "\ No newline at end of file"
    NoNewline,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub binary: bool,
    pub too_large: bool,
    pub truncated: bool,
    pub hunks: Vec<Hunk>,
}

/// Diff of a working tree file against HEAD (what committing it would record).
pub fn working(repo: &Path, path: &str, orig_path: Option<&str>, untracked: bool) -> Result<FileDiff> {
    if untracked {
        return untracked_file(&repo.join(path));
    }
    let base = if has_head(repo)? {
        "HEAD".to_string()
    } else {
        empty_tree(repo)?
    };
    let out = Git::new(repo)
        .args(["diff", "--no-color", "--no-ext-diff", "-M", &base, "--", path])
        .args(orig_path)
        .run()?;
    Ok(parse(&decode(&out.stdout)))
}

/// Diff of one file in a commit, against its first parent.
pub fn commit_file(
    repo: &Path,
    hash: &str,
    parent: Option<&str>,
    path: &str,
    orig_path: Option<&str>,
) -> Result<FileDiff> {
    let parent = match parent {
        Some(p) => p.to_string(),
        None => empty_tree(repo)?,
    };
    let out = Git::new(repo)
        .args(["diff", "--no-color", "--no-ext-diff", "-M", &parent, hash, "--", path])
        .args(orig_path)
        .run()?;
    Ok(parse(&decode(&out.stdout)))
}

fn untracked_file(file: &Path) -> Result<FileDiff> {
    let meta = std::fs::metadata(file)?;
    if meta.is_dir() {
        return Ok(FileDiff::default());
    }
    if meta.len() > MAX_UNTRACKED_BYTES {
        return Ok(FileDiff {
            too_large: true,
            ..Default::default()
        });
    }
    let bytes = std::fs::read(file)?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return Ok(FileDiff {
            binary: true,
            ..Default::default()
        });
    }
    let text = decode(&bytes);
    let mut diff = FileDiff::default();
    let mut lines = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i >= MAX_LINES {
            diff.truncated = true;
            break;
        }
        lines.push(DiffLine {
            kind: LineKind::Add,
            old_no: None,
            new_no: Some(i as u32 + 1),
            text: line.to_string(),
        });
    }
    if !lines.is_empty() {
        let header = format!("@@ -0,0 +1,{} @@", lines.len());
        diff.hunks.push(Hunk { header, lines });
    }
    Ok(diff)
}

/// Parses `git diff` unified output for a single file.
fn parse(text: &str) -> FileDiff {
    let mut diff = FileDiff::default();
    let mut old_no = 0u32;
    let mut new_no = 0u32;
    let mut count = 0usize;
    let mut in_hunks = false;

    for line in text.lines() {
        if !in_hunks {
            if line.starts_with("Binary files ") || line == "GIT binary patch" {
                diff.binary = true;
                return diff;
            }
            if !line.starts_with("@@") {
                continue;
            }
            in_hunks = true;
        }
        if line.starts_with("@@") {
            let (o, n) = parse_hunk_header(line);
            old_no = o;
            new_no = n;
            diff.hunks.push(Hunk {
                header: line.to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        if count >= MAX_LINES {
            diff.truncated = true;
            break;
        }
        let Some(hunk) = diff.hunks.last_mut() else { continue };
        let (kind, rest) = match line.as_bytes().first() {
            Some(b'+') => (LineKind::Add, &line[1..]),
            Some(b'-') => (LineKind::Delete, &line[1..]),
            Some(b'\\') => (LineKind::NoNewline, line),
            Some(b' ') => (LineKind::Context, &line[1..]),
            _ => (LineKind::Context, line),
        };
        let (o, n) = match kind {
            LineKind::Add => (None, Some(new_no)),
            LineKind::Delete => (Some(old_no), None),
            LineKind::Context => (Some(old_no), Some(new_no)),
            LineKind::NoNewline => (None, None),
        };
        match kind {
            LineKind::Add => new_no += 1,
            LineKind::Delete => old_no += 1,
            LineKind::Context => {
                old_no += 1;
                new_no += 1;
            }
            LineKind::NoNewline => {}
        }
        hunk.lines.push(DiffLine {
            kind,
            old_no: o,
            new_no: n,
            text: rest.to_string(),
        });
        count += 1;
    }
    diff
}

/// Extracts the starting line numbers from `@@ -a,b +c,d @@`.
fn parse_hunk_header(line: &str) -> (u32, u32) {
    let mut parts = line.split(' ').skip(1);
    let start = |s: Option<&str>| -> u32 { s.and_then(|s| s[1..].split(',').next()?.parse().ok()).unwrap_or(1) };
    let old = start(parts.next());
    let new = start(parts.next());
    (old, new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unified_diff() {
        let text = "diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -1,3 +1,3 @@ fn main\n a\n-b\n+c\n d\n\\ No newline at end of file\n";
        let d = parse(text);
        assert_eq!(d.hunks.len(), 1);
        let l = &d.hunks[0].lines;
        assert_eq!(l.len(), 5);
        assert_eq!((l[0].old_no, l[0].new_no), (Some(1), Some(1)));
        assert_eq!((l[1].kind, l[1].old_no), (LineKind::Delete, Some(2)));
        assert_eq!((l[2].kind, l[2].new_no), (LineKind::Add, Some(2)));
        assert_eq!((l[3].old_no, l[3].new_no), (Some(3), Some(3)));
        assert_eq!(l[4].kind, LineKind::NoNewline);
    }

    #[test]
    fn detects_binary() {
        let d = parse("diff --git a/x b/x\nBinary files a/x and b/x differ\n");
        assert!(d.binary);
    }
}
