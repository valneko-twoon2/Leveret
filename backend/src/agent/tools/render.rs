//! Turning git results into text and JSON for models: operation outcomes, diffs
//! (cut to a size limit) and timestamps.

use std::path::Path;

use serde_json::{Value, json};

use super::Output;
use crate::error::Result;
use crate::git::diff::{FileDiff, LineKind};
use crate::git::operation::Outcome;
use crate::git::status::ChangeKind;
use crate::git::{self, Git, decode};

pub(super) fn outcome(repo: &Path, outcome: Outcome) -> Result<Output> {
    Ok(Output::Json(match outcome {
        Outcome::Done => json!({ "outcome": "done", "status": brief_status(repo)? }),
        Outcome::Conflicts => json!({
            "outcome": "conflicts",
            "conflictedFiles": git::operation::conflicted_paths(repo)?,
            "next": "Fix the conflict markers in these files, call mark_resolved with their paths, then continue_operation (or abort_operation to cancel).",
        }),
    }))
}

pub(super) fn brief_status(repo: &Path) -> Result<Value> {
    let st = git::status::get(repo)?;
    Ok(json!({
        "branch": st.branch,
        "upstream": st.upstream,
        "ahead": st.ahead,
        "behind": st.behind,
        "changedFiles": st.files.len(),
        "operation": st.operation,
    }))
}

/// Cuts `text` at `max` bytes (on a character boundary) with a note.
fn truncate(mut text: String, max: usize) -> String {
    if text.len() > max {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n... (truncated; ask for a single path or a larger maxBytes)\n");
    }
    text
}

/// Diff of the working tree against HEAD, plus untracked files, as unified diff text.
pub(super) fn working_diff(repo: &Path, only: Option<&str>, max: usize) -> Result<String> {
    let st = git::status::get(repo)?;
    let mut out = String::new();
    for f in st
        .files
        .iter()
        .filter(|f| only.is_none_or(|p| f.path == p || f.orig_path.as_deref() == Some(p)))
    {
        if out.len() > max {
            break;
        }
        let untracked = f.kind == ChangeKind::Untracked;
        let d = git::diff::working(repo, &f.path, f.orig_path.as_deref(), untracked)?;
        render_file(&mut out, &f.path, f.orig_path.as_deref(), f.kind, &d);
    }
    if out.is_empty() {
        out = if only.is_some() {
            "No changes in this file.\n".into()
        } else {
            "No uncommitted changes.\n".into()
        };
    }
    Ok(truncate(out, max))
}

fn render_file(out: &mut String, path: &str, orig: Option<&str>, kind: ChangeKind, d: &FileDiff) {
    let old = orig.unwrap_or(path);
    out.push_str(&format!("diff --git a/{old} b/{path}\n"));
    match kind {
        ChangeKind::Untracked | ChangeKind::Added => out.push_str("new file\n--- /dev/null\n"),
        _ => out.push_str(&format!("--- a/{old}\n")),
    }
    if kind == ChangeKind::Deleted {
        out.push_str("+++ /dev/null\n");
    } else {
        out.push_str(&format!("+++ b/{path}\n"));
    }
    if d.binary {
        out.push_str("Binary file\n");
    } else if d.too_large {
        out.push_str("File too large to show\n");
    }
    for h in &d.hunks {
        out.push_str(&h.header);
        out.push('\n');
        for l in &h.lines {
            let prefix = match l.kind {
                LineKind::Context => ' ',
                LineKind::Add => '+',
                LineKind::Delete => '-',
                LineKind::NoNewline => '\\',
            };
            out.push(prefix);
            out.push_str(&l.text);
            out.push('\n');
        }
    }
}

pub(super) fn commit_diff(repo: &Path, hash: &str, max: usize) -> Result<String> {
    let out = Git::new(repo)
        .args([
            "show",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--format=",
            "-M",
            "--diff-merges=first-parent",
        ])
        .arg("--end-of-options")
        .arg(hash)
        .run()?;
    Ok(truncate(decode(&out.stdout), max))
}

/// A stash as unified diff text: tracked changes against the commit it was made
/// on, then untracked files stashed with it.
pub(super) fn stash_diff(repo: &Path, oid: &str, max: usize) -> Result<String> {
    let d = git::stash::detail(repo, oid)?;
    let diff = |from: &str, to: &str| {
        Git::new(repo)
            .args(["diff", "--no-color", "--no-ext-diff", "--no-textconv", "-M", from, to])
            .run()
    };
    let mut text = decode(&diff(&d.base, &d.oid)?.stdout);
    if let Some(u) = &d.untracked {
        text.push_str(&decode(&diff(&git::empty_tree(repo)?, u)?.stdout));
    }
    if text.is_empty() {
        text = "The stash holds no changes.\n".into();
    }
    Ok(truncate(text, max))
}

/// Unix seconds as an RFC 3339 UTC timestamp.
pub(super) fn iso8601(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(1_792_243_800), "2026-10-17T13:30:00Z");
        assert_eq!(iso8601(951_782_400), "2000-02-29T00:00:00Z");
    }
}
