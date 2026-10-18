//! Searching the history in git (not just the loaded commits) and the history
//! of one file, following renames.

use std::path::Path;

use serde::Deserialize;

use super::{LogEntry, LogScope, RECORD, parse_entry, parse_name_status, scope_args};
use crate::error::Result;
use crate::git::{Git, decode, has_head};

/// What to look for; every given condition must match.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogQuery {
    /// Text in the commit message (case-insensitive). A pasted hash finds its commit too.
    #[serde(default)]
    pub message: Option<String>,
    /// Text in the author's name or email (case-insensitive).
    #[serde(default)]
    pub author: Option<String>,
    /// Commits that add or remove this text (`git log -S`).
    #[serde(default)]
    pub content: Option<String>,
    /// Commits that touch this file or folder.
    #[serde(default)]
    pub path: Option<String>,
    /// The history of this file, following renames: each entry gets the file's path then.
    #[serde(default)]
    pub file: Option<String>,
}

fn given(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

impl LogQuery {
    pub fn is_empty(&self) -> bool {
        [&self.message, &self.author, &self.content, &self.path, &self.file]
            .iter()
            .all(|v| given(v).is_none())
    }
}

/// Commits matching `query`, newest first (without a graph: the results skip
/// commits, so lanes would not connect).
pub fn search(repo: &Path, scope: LogScope, limit: u32, query: &LogQuery) -> Result<Vec<LogEntry>> {
    let file = given(&query.file);
    let mut git = Git::new(repo)
        .args(["log", "--date-order", "--decorate=full", "--no-color"])
        .arg(super::format())
        .arg(format!("--max-count={limit}"));
    if let Some(text) = given(&query.message) {
        git = git.arg(format!("--grep={text}"));
    }
    if let Some(text) = given(&query.author) {
        git = git.arg(format!("--author={text}"));
    }
    if given(&query.message).is_some() || given(&query.author).is_some() {
        git = git.args(["--regexp-ignore-case", "--fixed-strings"]);
    }
    if let Some(text) = given(&query.content) {
        git = git.arg(format!("-S{text}"));
    }
    if file.is_some() {
        git = git.args(["--follow", "--name-status", "-z"]);
    }
    git = scope_args(git, scope).arg("--");
    if let Some(f) = file {
        git = git.arg(f);
    } else if let Some(p) = given(&query.path) {
        git = git.arg(p);
    }
    let out = match git.run() {
        Ok(out) => out,
        Err(_) if !has_head(repo)? => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let text = decode(&out.stdout);
    let mut entries: Vec<LogEntry> = text
        .split(RECORD)
        .filter_map(|record| {
            if file.is_none() {
                return parse_entry(record);
            }
            // `-z`: the formatted header ends with "\0\n", then the file's name status.
            let (header, files) = record.split_once("\0\n").unwrap_or((record, ""));
            let mut entry = parse_entry(header.trim_end_matches('\0'))?;
            entry.file_path = parse_name_status(files).pop().map(|f| f.path);
            Some(entry)
        })
        .collect();

    if let Some(text) = given(&query.message)
        && file.is_none()
        && text.len() >= 7
        && text.len() <= 64
        && text.bytes().all(|b| b.is_ascii_hexdigit())
        && !entries.iter().any(|e| e.hash.starts_with(&text.to_ascii_lowercase()))
        && let Some(entry) = by_hash(repo, text)?
    {
        entries.insert(0, entry);
    }
    Ok(entries)
}

/// The commit a (possibly abbreviated) hash names, if any.
fn by_hash(repo: &Path, hash: &str) -> Result<Option<LogEntry>> {
    let out = Git::new(repo)
        .args(["log", "-1", "--decorate=full", "--no-color"])
        .arg(super::format())
        .arg(format!("{hash}^{{commit}}"))
        .arg("--")
        .ok_codes(&[0, 128])
        .run()?;
    if out.code != 0 {
        return Ok(None);
    }
    Ok(decode(&out.stdout).split(RECORD).find_map(parse_entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, git_out, repo};

    fn subjects(entries: &[LogEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.subject.as_str()).collect()
    }

    #[test]
    fn searches_messages_authors_content_paths_and_files() {
        let dir = repo();
        let p = dir.path();
        std::fs::write(p.join("a.txt"), "one\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "最初の追加 [WIP]"]);
        std::fs::write(p.join("a.txt"), "one\nneedle\n").unwrap();
        git(p, &["commit", "-qam", "Add needle"]);
        std::fs::create_dir_all(p.join("dir")).unwrap();
        git(p, &["mv", "a.txt", "dir/b.txt"]);
        git(p, &["commit", "-qm", "move"]);
        std::fs::write(p.join("other.txt"), "x\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["-c", "user.name=Hana Sato", "commit", "-qm", "other"]);

        let q = |q: LogQuery| search(p, LogScope::Head, 100, &q).unwrap();
        // Literal and case-insensitive: "[wip]" is not a character class.
        let found = q(LogQuery {
            message: Some("[wip]".into()),
            ..Default::default()
        });
        assert_eq!(subjects(&found), ["最初の追加 [WIP]"]);
        assert_eq!(
            subjects(&q(LogQuery {
                author: Some("hana".into()),
                ..Default::default()
            })),
            ["other"]
        );
        assert_eq!(
            subjects(&q(LogQuery {
                content: Some("needle".into()),
                ..Default::default()
            })),
            ["Add needle"]
        );
        assert_eq!(
            subjects(&q(LogQuery {
                path: Some("dir".into()),
                ..Default::default()
            })),
            ["move"]
        );

        // The file's history goes back through the rename, with the path at each commit.
        let history = q(LogQuery {
            file: Some("dir/b.txt".into()),
            ..Default::default()
        });
        assert_eq!(subjects(&history), ["move", "Add needle", "最初の追加 [WIP]"]);
        let paths: Vec<_> = history.iter().map(|e| e.file_path.as_deref().unwrap()).collect();
        assert_eq!(paths, ["dir/b.txt", "a.txt", "a.txt"]);
        assert!(history.iter().all(|e| e.graph.edges.is_empty()));

        // A pasted hash finds its commit, even when the message doesn't match.
        let hash = git_out(p, &["rev-parse", "HEAD~2"]);
        let found = q(LogQuery {
            message: Some(hash[..10].into()),
            ..Default::default()
        });
        assert_eq!(subjects(&found), ["Add needle"]);
        assert!(
            q(LogQuery {
                message: Some("nothing like this".into()),
                ..Default::default()
            })
            .is_empty()
        );
    }
}
