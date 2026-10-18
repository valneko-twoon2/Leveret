//! Partial commits and discards: patches built in Rust from the lines a user
//! picked in a working tree diff. The webview only sends line positions plus the
//! diff's token; patch text never comes from it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::diff::{self, LineKind, RawDiff, RawHunk};
use super::{Git, PathspecFile, decode};
use crate::error::{AppError, Result};

/// A line in a diff: (hunk index, line index in that hunk), as in `FileDiff.hunks`.
pub type LineRef = (u32, u32);

/// Some lines of one file, for a partial commit or discard.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineSelection {
    pub path: String,
    #[serde(default)]
    pub untracked: bool,
    /// `FileDiff.token` of the diff the lines were picked in.
    pub token: String,
    /// `[hunk, line]` pairs; context lines are ignored.
    pub lines: Vec<[u32; 2]>,
}

impl LineSelection {
    fn picked(&self) -> BTreeSet<LineRef> {
        self.lines.iter().map(|&[h, l]| (h, l)).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Apply to HEAD's version: unpicked additions are left out, unpicked deletions kept.
    Stage,
    /// Apply in reverse to the working tree file: only the picked changes are undone.
    Discard,
}

/// The current diff of a file, checked against the token the lines were picked in.
enum Current {
    Tracked(Vec<u8>),
    Untracked(Vec<u8>),
}

fn current(repo: &Path, sel: &LineSelection) -> Result<Current> {
    let current = if sel.untracked {
        let file = repo.join(&sel.path);
        // A link to some other file is not a file of the repository.
        if !std::fs::symlink_metadata(&file)?.is_file() {
            return Err(AppError::InvalidInput(format!("'{}' is not a regular file", sel.path)));
        }
        Current::Untracked(std::fs::read(&file)?)
    } else {
        let context = diff::token_context(&sel.token).ok_or_else(|| AppError::DiffChanged(sel.path.clone()))?;
        Current::Tracked(diff::raw_working(repo, &sel.path, None, false, context)?)
    };
    let bytes = match &current {
        Current::Tracked(b) | Current::Untracked(b) => b,
    };
    let context = diff::token_context(&sel.token).unwrap_or(diff::DEFAULT_CONTEXT);
    if diff::token(bytes, context) != sel.token {
        return Err(AppError::DiffChanged(sel.path.clone()));
    }
    let selectable = match &current {
        Current::Tracked(raw) => diff::tracked_selectable(raw),
        Current::Untracked(bytes) => diff::untracked_selectable(bytes),
    };
    if !selectable {
        return Err(AppError::InvalidInput(format!(
            "lines of '{}' cannot be picked",
            sel.path
        )));
    }
    Ok(current)
}

/// Every added or deleted line of the given hunks of a file's current diff, for
/// agents (which pick whole hunks by number). Returns the selection to use.
pub fn select_hunks(repo: &Path, path: &str, untracked: bool, hunks: &[u32]) -> Result<LineSelection> {
    let d = diff::working(repo, path, None, untracked)?;
    if !d.selectable {
        return Err(AppError::InvalidInput(format!("hunks of '{path}' cannot be picked")));
    }
    let mut lines = Vec::new();
    for &h in hunks {
        let hunk = d
            .hunks
            .get(h as usize)
            .ok_or_else(|| AppError::InvalidInput(format!("'{path}' has no hunk {h}")))?;
        for (i, line) in hunk.lines.iter().enumerate() {
            if matches!(line.kind, LineKind::Add | LineKind::Delete) {
                lines.push([h, i as u32]);
            }
        }
    }
    Ok(LineSelection {
        path: path.to_string(),
        untracked,
        token: d.token,
        lines,
    })
}

/// Adds the picked lines of each file to the index at `index` (built from HEAD).
pub(crate) fn stage_into(repo: &Path, index: &Path, selections: &[LineSelection]) -> Result<()> {
    let env = || [("GIT_INDEX_FILE".to_string(), index.display().to_string())];
    let mut patch = Vec::new();
    for sel in selections {
        match current(repo, sel)? {
            Current::Tracked(raw) => patch.extend(build(&raw, &sel.picked(), Mode::Stage)?.unwrap_or_default()),
            Current::Untracked(bytes) => {
                // A new file can't be patched into the index: write the picked lines as
                // a blob (filtered like `git add` would, for line endings) and add that.
                let content = untracked_content(&bytes, &sel.picked(), Mode::Stage)?;
                let oid = Git::new(repo)
                    .args(["hash-object", "-w", "--stdin"])
                    .arg(format!("--path={}", sel.path))
                    .stdin(content)
                    .run_string()?;
                let mut info = format!("100644 {}\t{}", oid.trim(), sel.path).into_bytes();
                info.push(0);
                Git::new(repo)
                    .envs(env())
                    .args(["update-index", "--add", "-z", "--index-info"])
                    .stdin(info)
                    .run()?;
            }
        }
    }
    if !patch.is_empty() {
        Git::new(repo)
            .envs(env())
            .args(["apply", "--cached", "--recount", "--whitespace=nowarn", "-"])
            .stdin(patch)
            .run()?;
    }
    Ok(())
}

/// Undoes the picked lines in the working tree file. The current file goes to the
/// Recycle Bin first (as a copy), so the discard can be undone.
pub fn discard(repo: &Path, sel: &LineSelection) -> Result<()> {
    discard_with(repo, sel, |files| {
        trash::delete_all(files).map_err(|e| AppError::Other(e.to_string()))
    })
}

/// [`discard`] with a custom way of removing the backup (tests don't use the Recycle Bin).
pub(crate) fn discard_with(
    repo: &Path,
    sel: &LineSelection,
    remove: impl FnOnce(&[PathBuf]) -> Result<()>,
) -> Result<()> {
    let picked = sel.picked();
    let current = current(repo, sel)?;
    let file = repo.join(&sel.path);
    let backup_dir = tempfile::tempdir()?;
    let name = file
        .file_name()
        .ok_or_else(|| AppError::InvalidInput(sel.path.clone()))?;
    let backup = backup_dir.path().join(name);
    std::fs::copy(&file, &backup)?;

    match current {
        Current::Tracked(raw) => {
            let Some(patch) = build(&raw, &picked, Mode::Discard)? else {
                return Ok(());
            };
            remove(&[backup])?;
            Git::new(repo)
                .args(["apply", "-R", "--recount", "--whitespace=nowarn", "-"])
                .stdin(patch)
                .run()?;
        }
        Current::Untracked(bytes) => {
            let kept = untracked_content(&bytes, &picked, Mode::Discard)?;
            if kept.len() == bytes.len() {
                return Ok(());
            }
            remove(&[backup])?;
            // Written next to the file, then renamed over it.
            let dir = file.parent().ok_or_else(|| AppError::InvalidInput(sel.path.clone()))?;
            let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
            std::io::Write::write_all(&mut tmp, &kept)?;
            tmp.persist(&file).map_err(|e| AppError::Io(e.error))?;
        }
    }
    Ok(())
}

/// Commits the given paths from a temporary index: HEAD, plus `full` paths as
/// they are in the working tree, plus the picked lines. Hooks and signing run as
/// for any commit. The real index is then reset to the new commit for those paths.
pub(crate) fn commit_from_index(
    repo: &Path,
    full: &[String],
    selections: &[LineSelection],
    message: &str,
    amend: bool,
) -> Result<()> {
    let dir = tempfile::tempdir()?;
    // Must not exist yet: an empty file is not a valid index, a missing one is empty.
    let index = dir.path().join("index");
    let env = || [("GIT_INDEX_FILE".to_string(), index.display().to_string())];
    if super::has_head(repo)? {
        Git::new(repo).envs(env()).args(["read-tree", "HEAD"]).run()?;
    }
    if !full.is_empty() {
        let spec = PathspecFile::new(full)?;
        Git::new(repo)
            .envs(env())
            .args(["add", "--all"])
            .args(spec.args())
            .run()?;
    }
    stage_into(repo, &index, selections)?;

    let mut git = Git::new(repo).envs(env()).args(["commit", "--file=-"]).stdin(message);
    if amend {
        git = git.arg("--amend");
    }
    git.run().map_err(AppError::classify_git)?;

    let mut paths: Vec<&str> = full.iter().map(String::as_str).collect();
    paths.extend(selections.iter().map(|s| s.path.as_str()));
    let spec = PathspecFile::new(&paths)?;
    Git::new(repo).args(["reset", "-q"]).args(spec.args()).run()?;
    Ok(())
}

/// Changes inside one block of consecutive added / deleted lines are kept together
/// when the block changes whether the old file ends with a newline: those can't be
/// split into a valid patch.
fn whole_blocks(kinds: &[LineKind], picked: &mut [bool]) {
    let mut start = 0;
    while start < kinds.len() {
        if kinds[start] == LineKind::Context {
            start += 1;
            continue;
        }
        let end = (start..kinds.len())
            .find(|&i| kinds[i] == LineKind::Context)
            .unwrap_or(kinds.len());
        let old_side_eof =
            (start + 1..end).any(|i| kinds[i] == LineKind::NoNewline && kinds[i - 1] == LineKind::Delete);
        if old_side_eof && picked[start..end].iter().any(|&p| p) {
            for (i, p) in picked.iter_mut().enumerate().take(end).skip(start) {
                *p = matches!(kinds[i], LineKind::Add | LineKind::Delete);
            }
        }
        start = end;
    }
}

/// Builds a patch with the picked lines of a tracked file's raw diff, or None
/// when nothing is picked.
pub(crate) fn build(raw_bytes: &[u8], picked: &BTreeSet<LineRef>, mode: Mode) -> Result<Option<Vec<u8>>> {
    let raw = diff::split(raw_bytes);
    if let Some(&(h, l)) = picked.iter().find(|&&(h, l)| {
        raw.hunks
            .get(h as usize)
            .is_none_or(|hunk| l as usize >= hunk.lines.len())
    }) {
        return Err(AppError::InvalidInput(format!("line {h}:{l} is not in the diff")));
    }
    let mut body = Vec::new();
    let mut delta: i64 = 0;
    for (h, hunk) in raw.hunks.iter().enumerate() {
        let lines: Vec<(LineKind, &[u8])> = hunk.lines.iter().map(|l| diff::line_kind(l)).collect();
        let kinds: Vec<LineKind> = lines.iter().map(|l| l.0).collect();
        let mut sel: Vec<bool> = (0..lines.len())
            .map(|l| matches!(kinds[l], LineKind::Add | LineKind::Delete) && picked.contains(&(h as u32, l as u32)))
            .collect();
        whole_blocks(&kinds, &mut sel);
        if sel.iter().any(|&s| s) {
            build_hunk(hunk, &lines, &sel, mode, &mut delta, &mut body)?;
        }
    }
    if body.is_empty() {
        return Ok(None);
    }
    let mut patch = match mode {
        Mode::Stage => stage_header(&raw),
        Mode::Discard => modification_header(&raw),
    };
    patch.extend(body);
    Ok(Some(patch))
}

fn build_hunk(
    hunk: &RawHunk<'_>,
    lines: &[(LineKind, &[u8])],
    sel: &[bool],
    mode: Mode,
    delta: &mut i64,
    out: &mut Vec<u8>,
) -> Result<()> {
    let mut text = Vec::new();
    let (mut old_count, mut new_count) = (0i64, 0i64);
    // Whether the previous line was left out (a following "\ No newline" goes with it).
    let mut dropped = false;
    for (i, &(kind, rest)) in lines.iter().enumerate() {
        let sign = match (kind, sel[i], mode) {
            (LineKind::NoNewline, ..) => {
                if !dropped {
                    push_line(&mut text, None, rest);
                }
                continue;
            }
            (LineKind::Add, true, _) => b'+',
            (LineKind::Delete, true, _) => b'-',
            (LineKind::Add, false, Mode::Stage) | (LineKind::Delete, false, Mode::Discard) => {
                dropped = true;
                continue;
            }
            // Context, and unpicked changes that stay as they are.
            _ => b' ',
        };
        push_line(&mut text, Some(sign), rest);
        dropped = false;
        if sign != b'+' {
            old_count += 1;
        }
        if sign != b'-' {
            new_count += 1;
        }
    }

    let (old_start, old_len, new_start, new_len) = parse_ranges(hunk.header)?;
    // Unified diff ranges of length 0 name the line before the (empty) range.
    let first = |start: i64, len: i64| if len == 0 { start + 1 } else { start };
    let start = |first: i64, len: i64| if len == 0 { first - 1 } else { first };
    let (o, n) = match mode {
        Mode::Stage => (old_start, start(first(old_start, old_len) + *delta, new_count)),
        Mode::Discard => (start(first(new_start, new_len) - *delta, old_count), new_start),
    };
    *delta += new_count - old_count;
    out.extend_from_slice(format!("@@ -{o},{old_count} +{n},{new_count} @@\n").as_bytes());
    out.extend(text);
    Ok(())
}

fn push_line(out: &mut Vec<u8>, sign: Option<u8>, rest: &[u8]) {
    out.extend(sign);
    out.extend_from_slice(rest);
    out.push(b'\n');
}

/// `@@ -a,b +c,d @@` as (a, b, c, d); a missing length means 1.
fn parse_ranges(header: &[u8]) -> Result<(i64, i64, i64, i64)> {
    let text = decode(diff::trim_cr(header));
    let bad = || AppError::Other(format!("unexpected hunk header: {text}"));
    let mut parts = text.split(' ').skip(1);
    let mut range = |sign: char| -> Result<(i64, i64)> {
        let part = parts.next().and_then(|p| p.strip_prefix(sign)).ok_or_else(bad)?;
        let (start, len) = part.split_once(',').unwrap_or((part, "1"));
        Ok((start.parse().map_err(|_| bad())?, len.parse().map_err(|_| bad())?))
    };
    let (a, b) = range('-')?;
    let (c, d) = range('+')?;
    Ok((a, b, c, d))
}

/// git's own header without the `index` line (blob ids that no longer match).
fn stage_header(raw: &RawDiff<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    for line in raw.header.iter().filter(|l| !l.starts_with(b"index ")) {
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    out
}

/// A plain modification header (`--- a/P`, `+++ b/P`) for a file that exists in
/// the working tree, even when the diff shows it as new.
fn modification_header(raw: &RawDiff<'_>) -> Vec<u8> {
    let name = |prefix: &[u8]| {
        raw.header
            .iter()
            .find_map(|l| l.strip_prefix(prefix))
            .unwrap_or_default()
    };
    // `a/x` -> `b/x`, also inside a quoted name (`"a/x"`).
    let swap = |n: &[u8], from: u8, to: u8| -> Vec<u8> {
        let mut n = n.to_vec();
        let at = usize::from(n.first() == Some(&b'"'));
        if n.get(at) == Some(&from) && n.get(at + 1) == Some(&b'/') {
            n[at] = to;
        }
        n
    };
    let (old, new) = (name(b"--- "), name(b"+++ "));
    let null = |n: &[u8]| diff::trim_cr(n) == b"/dev/null" || n.is_empty();
    let old = if null(old) { swap(new, b'b', b'a') } else { old.to_vec() };
    let new = if null(new) {
        swap(&old, b'a', b'b')
    } else {
        new.to_vec()
    };
    let mut out = Vec::new();
    if let Some(first) = raw.header.first().filter(|l| l.starts_with(b"diff --git ")) {
        out.extend_from_slice(first);
        out.push(b'\n');
    }
    for (prefix, n) in [(&b"--- "[..], old), (b"+++ ", new)] {
        out.extend_from_slice(prefix);
        out.extend(n);
        out.push(b'\n');
    }
    out
}

/// An untracked file's content with only the picked lines (Stage) or without them (Discard).
fn untracked_content(bytes: &[u8], picked: &BTreeSet<LineRef>, mode: Mode) -> Result<Vec<u8>> {
    let lines = diff::untracked_lines(bytes).ok_or_else(|| AppError::InvalidInput("binary file".into()))?;
    if let Some(&(h, l)) = picked.iter().find(|&&(h, l)| h != 0 || l as usize >= lines.len()) {
        return Err(AppError::InvalidInput(format!("line {h}:{l} is not in the file")));
    }
    let ends_with_newline = bytes.ends_with(b"\n");
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if picked.contains(&(0, i as u32)) == (mode == Mode::Stage) {
            out.extend_from_slice(line);
            if i + 1 < lines.len() || ends_with_newline {
                out.push(b'\n');
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
