//! Details of one commit: message, people and changed files.

use std::path::Path;

use serde::Serialize;

use super::FIELD;
use crate::error::Result;
use crate::git::status::ChangeKind;
use crate::git::{Git, decode, join};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    pub orig_path: Option<String>,
    pub kind: ChangeKind,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
    pub hash: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub author_time: i64,
    pub committer_name: String,
    pub committer_email: String,
    pub commit_time: i64,
    pub message: String,
    pub files: Vec<CommitFile>,
}

pub fn detail(repo: &Path, hash: &str) -> Result<CommitDetail> {
    let format = format!("--format=%H{FIELD}%P{FIELD}%an{FIELD}%ae{FIELD}%at{FIELD}%cn{FIELD}%ce{FIELD}%ct{FIELD}%B");
    // The message and the changed files come from two processes run in parallel; the
    // file list is diffed against the first parent (`<hash>^1`), which root commits lack.
    let (text, diff) = std::thread::scope(|s| {
        let diff = s.spawn(|| {
            Git::new(repo)
                .args(["diff-tree", "-r", "-M", "--name-status", "-z", "--no-commit-id"])
                .args([format!("{hash}^1"), hash.to_string()])
                .run()
        });
        let text = Git::new(repo)
            .args(["show", "-s", "--no-color"])
            .arg(format)
            .arg(hash)
            .arg("--")
            .run_string();
        (text, join(diff))
    });
    let text = text?;
    let f: Vec<&str> = text.splitn(9, FIELD).collect();
    let [h, parents, an, ae, at, cn, ce, ct, body] = f[..] else {
        return Err(crate::error::AppError::Other(format!(
            "unexpected git show output for {hash}"
        )));
    };
    let parents: Vec<String> = parents.split_whitespace().map(String::from).collect();

    // Files changed relative to the first parent (or everything, for a root commit).
    let out = match parents.first() {
        Some(_) => diff?,
        None => Git::new(repo)
            .args([
                "diff-tree",
                "-r",
                "-M",
                "--name-status",
                "-z",
                "--no-commit-id",
                "--root",
                h,
            ])
            .run()?,
    };
    let files = parse_name_status(&decode(&out.stdout));

    Ok(CommitDetail {
        hash: h.into(),
        parents,
        author_name: an.into(),
        author_email: ae.into(),
        author_time: at.parse().unwrap_or(0),
        committer_name: cn.into(),
        committer_email: ce.into(),
        commit_time: ct.parse().unwrap_or(0),
        message: body.trim_end().into(),
        files,
    })
}

/// Parses `--name-status -z` output: `M\0path\0`, `R100\0old\0new\0`.
pub(crate) fn parse_name_status(text: &str) -> Vec<CommitFile> {
    let mut files = Vec::new();
    let mut tokens = text.split('\0').filter(|t| !t.is_empty());
    while let Some(status) = tokens.next() {
        let kind = match status.as_bytes()[0] {
            b'A' => ChangeKind::Added,
            b'D' => ChangeKind::Deleted,
            b'R' => ChangeKind::Renamed,
            b'C' => ChangeKind::Copied,
            b'T' => ChangeKind::TypeChanged,
            b'U' => ChangeKind::Conflicted,
            _ => ChangeKind::Modified,
        };
        let (path, orig_path) = if matches!(kind, ChangeKind::Renamed | ChangeKind::Copied) {
            let old = tokens.next().unwrap_or_default();
            (tokens.next().unwrap_or_default().to_string(), Some(old.to_string()))
        } else {
            (tokens.next().unwrap_or_default().to_string(), None)
        };
        files.push(CommitFile { path, orig_path, kind });
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_status() {
        let f = parse_name_status("M\0a.txt\0R100\0old.txt\0new.txt\0A\0日本語.txt\0");
        assert_eq!(f.len(), 3);
        assert_eq!(f[1].path, "new.txt");
        assert_eq!(f[1].orig_path.as_deref(), Some("old.txt"));
        assert_eq!(f[2].kind, ChangeKind::Added);
    }
}
