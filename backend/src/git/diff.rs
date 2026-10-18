//! Diffs of working tree files and of files in commits, parsed for display.
//!
//! The raw `git diff` output is split into lines once ([`split`]); both the display
//! parser and the patch builder (`git::patch`) work from that split, so the lines a
//! user picks in the UI are exactly the lines a partial commit or discard uses.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use serde::Serialize;

use super::{Git, decode, empty_tree, has_head};
use crate::error::Result;

/// Larger diffs are cut off to keep the UI responsive.
pub(crate) const MAX_LINES: usize = 20_000;
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
    /// Whether lines can be picked for a partial commit or discard (see `git::patch`).
    pub selectable: bool,
    /// Identifies the exact diff shown, so a selection made on an older one is refused.
    pub token: String,
}

/// A diff split into lines (without the `\n`; a `\r` stays).
pub(crate) struct RawDiff<'a> {
    /// Lines before the first hunk (`diff --git`, `index`, `---`, `+++`, ...).
    pub header: Vec<&'a [u8]>,
    pub hunks: Vec<RawHunk<'a>>,
    pub binary: bool,
}

pub(crate) struct RawHunk<'a> {
    /// The `@@ -a,b +c,d @@` line.
    pub header: &'a [u8],
    pub lines: Vec<&'a [u8]>,
}

pub(crate) fn split(bytes: &[u8]) -> RawDiff<'_> {
    let mut diff = RawDiff {
        header: Vec::new(),
        hunks: Vec::new(),
        binary: false,
    };
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if body.is_empty() {
        return diff;
    }
    for line in body.split(|&b| b == b'\n') {
        if line.starts_with(b"@@") {
            diff.hunks.push(RawHunk {
                header: line,
                lines: Vec::new(),
            });
        } else if let Some(hunk) = diff.hunks.last_mut() {
            hunk.lines.push(line);
        } else {
            if line.starts_with(b"Binary files ") || trim_cr(line) == b"GIT binary patch" {
                diff.binary = true;
            }
            diff.header.push(line);
        }
    }
    diff
}

pub(crate) fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// The kind of a hunk line and its text without the leading sign.
pub(crate) fn line_kind(line: &[u8]) -> (LineKind, &[u8]) {
    match line.first() {
        Some(b'+') => (LineKind::Add, &line[1..]),
        Some(b'-') => (LineKind::Delete, &line[1..]),
        Some(b'\\') => (LineKind::NoNewline, line),
        Some(b' ') => (LineKind::Context, &line[1..]),
        // An empty context line (`diff.suppressBlankEmpty`).
        _ => (LineKind::Context, line),
    }
}

/// Lines of context around each change, as `git diff` shows by default.
pub const DEFAULT_CONTEXT: u32 = 3;

/// Identifies diff output; equal tokens mean the same lines. It starts with the
/// number of context lines the diff was made with, so that picked lines can be
/// matched against the same diff again (the webview never has to send it back).
pub(crate) fn token(bytes: &[u8], context: u32) -> String {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    format!("{context}.{:016x}", h.finish())
}

/// The context line count a token was made with.
pub(crate) fn token_context(token: &str) -> Option<u32> {
    token.split_once('.')?.0.parse().ok().filter(|&c| c <= MAX_CONTEXT)
}

/// The most context lines asked for (`security::diff_context` enforces the same).
pub const MAX_CONTEXT: u32 = 100;

/// Diff of a working tree file against HEAD (what committing it would record).
pub fn working(repo: &Path, path: &str, orig_path: Option<&str>, untracked: bool) -> Result<FileDiff> {
    working_with(repo, path, orig_path, untracked, false, DEFAULT_CONTEXT)
}

/// `-w`: leave out changes that only touch whitespace (the user's diff setting).
fn whitespace_arg(ignore_whitespace: bool) -> Option<&'static str> {
    ignore_whitespace.then_some("--ignore-all-space")
}

/// Raw `git diff` of a tracked working tree file against HEAD. Explicit prefixes
/// and no textconv keep the output appliable as a patch whatever the user's config.
pub(crate) fn raw_working(
    repo: &Path,
    path: &str,
    orig_path: Option<&str>,
    ignore_whitespace: bool,
    context: u32,
) -> Result<Vec<u8>> {
    let diff = |base: &str| {
        Git::new(repo)
            .args(["diff", "--no-color", "--no-ext-diff", "--no-textconv", "-M"])
            .arg(format!("-U{context}"))
            .args(["--src-prefix=a/", "--dst-prefix=b/"])
            .args(whitespace_arg(ignore_whitespace))
            .args([base, "--", path])
            .args(orig_path)
            .run()
    };
    // HEAD exists except in a new repository; only check after a failure, to
    // spare a git process (tens of milliseconds) on every diff.
    let out = match diff("HEAD") {
        Ok(out) => out,
        Err(_) if !has_head(repo)? => diff(&empty_tree(repo)?)?,
        Err(e) => return Err(e),
    };
    Ok(out.stdout)
}

/// Like [`working`]; `ignore_whitespace` hides whitespace-only changes and `context`
/// is the number of unchanged lines shown around each change.
pub fn working_with(
    repo: &Path,
    path: &str,
    orig_path: Option<&str>,
    untracked: bool,
    ignore_whitespace: bool,
    context: u32,
) -> Result<FileDiff> {
    if untracked {
        return untracked_file(&repo.join(path), context);
    }
    let raw = raw_working(repo, path, orig_path, ignore_whitespace, context)?;
    let mut diff = parse(&raw);
    diff.selectable = !ignore_whitespace && orig_path.is_none() && lines_selectable(&split(&raw), &diff);
    diff.token = token(&raw, context);
    Ok(diff)
}

/// Whether a tracked file's diff can be committed or discarded line by line:
/// modified and added text files only (not deleted, renamed or mode-only changes).
fn lines_selectable(raw: &RawDiff<'_>, diff: &FileDiff) -> bool {
    let unsupported = [&b"deleted file mode"[..], b"rename from", b"copy from"];
    !diff.binary
        && !diff.truncated
        && !diff.hunks.is_empty()
        && !raw.header.iter().any(|l| unsupported.iter().any(|u| l.starts_with(u)))
}

/// [`FileDiff::selectable`] for a tracked file's raw diff (from [`raw_working`] without `-w`).
pub(crate) fn tracked_selectable(raw: &[u8]) -> bool {
    lines_selectable(&split(raw), &parse(raw))
}

/// [`FileDiff::selectable`] for an untracked file's content.
pub(crate) fn untracked_selectable(bytes: &[u8]) -> bool {
    bytes.len() as u64 <= MAX_UNTRACKED_BYTES
        && untracked_lines(bytes).is_some_and(|l| !l.is_empty() && l.len() <= MAX_LINES)
}

/// Diff of one file in a commit, against its first parent.
#[cfg(test)]
pub fn commit_file(
    repo: &Path,
    hash: &str,
    parent: Option<&str>,
    path: &str,
    orig_path: Option<&str>,
) -> Result<FileDiff> {
    commit_file_with(repo, hash, parent, path, orig_path, false, DEFAULT_CONTEXT)
}

/// Like [`commit_file`]; `ignore_whitespace` hides whitespace-only changes and
/// `context` is the number of unchanged lines shown around each change.
pub fn commit_file_with(
    repo: &Path,
    hash: &str,
    parent: Option<&str>,
    path: &str,
    orig_path: Option<&str>,
    ignore_whitespace: bool,
    context: u32,
) -> Result<FileDiff> {
    let parent = match parent {
        Some(p) => p.to_string(),
        None => empty_tree(repo)?,
    };
    let out = Git::new(repo)
        .args(["diff", "--no-color", "--no-ext-diff", "-M"])
        .arg(format!("-U{context}"))
        .args(whitespace_arg(ignore_whitespace))
        .args([&parent, hash, "--", path])
        .args(orig_path)
        .run()?;
    Ok(parse(&out.stdout))
}

/// The lines of an untracked file (as shown: one added line each), or None when
/// it is shown as binary or too large instead.
pub(crate) fn untracked_lines(bytes: &[u8]) -> Option<Vec<&[u8]>> {
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return None;
    }
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if body.is_empty() {
        return Some(Vec::new());
    }
    Some(body.split(|&b| b == b'\n').collect())
}

fn untracked_file(file: &Path, context: u32) -> Result<FileDiff> {
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
    let Some(all) = untracked_lines(&bytes) else {
        return Ok(FileDiff {
            binary: true,
            ..Default::default()
        });
    };
    let mut diff = FileDiff {
        truncated: all.len() > MAX_LINES,
        token: token(&bytes, context),
        ..Default::default()
    };
    let lines: Vec<DiffLine> = all
        .iter()
        .take(MAX_LINES)
        .enumerate()
        .map(|(i, line)| DiffLine {
            kind: LineKind::Add,
            old_no: None,
            new_no: Some(i as u32 + 1),
            text: decode(trim_cr(line)),
        })
        .collect();
    if !lines.is_empty() {
        let header = format!("@@ -0,0 +1,{} @@", lines.len());
        diff.hunks.push(Hunk { header, lines });
    }
    diff.selectable = !diff.truncated && !diff.hunks.is_empty();
    Ok(diff)
}

/// Parses `git diff` unified output for a single file.
fn parse(bytes: &[u8]) -> FileDiff {
    let raw = split(bytes);
    let mut diff = FileDiff {
        binary: raw.binary,
        ..Default::default()
    };
    if raw.binary {
        return diff;
    }
    let mut count = 0usize;
    for raw_hunk in &raw.hunks {
        let header = decode(trim_cr(raw_hunk.header));
        let (mut old_no, mut new_no) = parse_hunk_header(&header);
        let mut hunk = Hunk {
            header,
            lines: Vec::new(),
        };
        for line in &raw_hunk.lines {
            if count >= MAX_LINES {
                diff.truncated = true;
                break;
            }
            let (kind, text) = line_kind(line);
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
                text: decode(trim_cr(text)),
            });
            count += 1;
        }
        diff.hunks.push(hunk);
        if diff.truncated {
            break;
        }
    }
    diff
}

/// Extracts the starting line numbers from `@@ -a,b +c,d @@`.
fn parse_hunk_header(line: &str) -> (u32, u32) {
    let mut parts = line.split(' ').skip(1);
    let start = |s: Option<&str>| -> u32 { s.and_then(|s| s.get(1..)?.split(',').next()?.parse().ok()).unwrap_or(1) };
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
        let d = parse(text.as_bytes());
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
    fn keeps_crlf_out_of_the_text_and_decodes_each_line() {
        let mut bytes = b"--- a/x\n+++ b/x\n@@ -1 +1,2 @@\n-a\r\n+".to_vec();
        bytes.extend_from_slice(&[0x82, 0xa0]); // "あ" in Shift_JIS
        bytes.extend_from_slice("\r\n+\u{3044}\r\n".as_bytes());
        let d = parse(&bytes);
        let texts: Vec<&str> = d.hunks[0].lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["a", "\u{3042}", "\u{3044}"]);
        // The raw split keeps the carriage returns for patches.
        assert_eq!(split(&bytes).hunks[0].lines[0], b"-a\r");
    }

    #[test]
    fn detects_binary() {
        let d = parse(b"diff --git a/x b/x\nBinary files a/x and b/x differ\n");
        assert!(d.binary);
    }

    #[test]
    fn untracked_lines_follow_the_display() {
        assert_eq!(untracked_lines(b"a\r\nb").unwrap(), [&b"a\r"[..], b"b"]);
        assert_eq!(untracked_lines(b"a\n\n").unwrap(), [&b"a"[..], b""]);
        assert!(untracked_lines(b"").unwrap().is_empty());
        assert!(untracked_lines(b"a\0b").is_none());
    }
}
