//! The commit log (with its graph and ref labels), searching it, and commit details.

use std::path::Path;

use serde::Serialize;

use super::{Git, decode, has_head};
use crate::error::Result;

mod detail;
mod graph;
mod search;

pub(crate) use detail::parse_name_status;
pub use detail::{CommitDetail, CommitFile, detail};
use graph::build_graph;
pub use search::{LogQuery, search};

pub(super) const FIELD: char = '\x1f';
pub(super) const RECORD: char = '\x1e';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefKind {
    /// Detached HEAD.
    Head,
    Branch,
    Remote,
    Tag,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefLabel {
    pub name: String,
    pub kind: RefKind,
    /// The branch HEAD points at.
    pub current: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    /// From the top of the row to the commit node (or straight through).
    Up,
    /// From the commit node (or straight through) to the bottom of the row.
    Down,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from: u16,
    pub to: u16,
    pub kind: EdgeKind,
    pub color: u16,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRow {
    pub col: u16,
    pub color: u16,
    pub width: u16,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub hash: String,
    pub parents: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    /// Unix seconds.
    pub author_time: i64,
    pub refs: Vec<RefLabel>,
    pub subject: String,
    pub graph: GraphRow,
    /// In a file's history: the file's path in this commit (it may have been renamed since).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogScope {
    /// History of HEAD only.
    Head,
    /// All local branches, remote branches and tags.
    All,
}

/// The `--format` that [`parse_entry`] reads.
fn format() -> String {
    format!("--format={RECORD}%H{FIELD}%P{FIELD}%an{FIELD}%ae{FIELD}%at{FIELD}%D{FIELD}%s")
}

fn scope_args(git: Git<'_>, scope: LogScope) -> Git<'_> {
    match scope {
        LogScope::Head => git.arg("HEAD"),
        LogScope::All => git.args(["HEAD", "--branches", "--remotes", "--tags"]),
    }
}

pub fn get(repo: &Path, scope: LogScope, limit: u32) -> Result<Vec<LogEntry>> {
    let git = Git::new(repo)
        .args(["log", "--date-order", "--decorate=full", "--no-color"])
        .arg(format())
        .arg(format!("--max-count={limit}"));
    let out = match scope_args(git, scope).arg("--").run() {
        Ok(out) => out,
        // A repository without commits has no HEAD to log. Checked only on failure,
        // to save a process on every call.
        Err(_) if !has_head(repo)? => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let text = decode(&out.stdout);

    let mut entries: Vec<LogEntry> = text.split(RECORD).filter_map(parse_entry).collect();
    build_graph(&mut entries);
    Ok(entries)
}

fn parse_entry(record: &str) -> Option<LogEntry> {
    let record = record.trim_end_matches('\n');
    if record.is_empty() {
        return None;
    }
    let f: Vec<&str> = record.splitn(7, FIELD).collect();
    let [hash, parents, an, ae, at, refs, subject] = f[..] else {
        return None;
    };
    Some(LogEntry {
        hash: hash.into(),
        parents: parents.split_whitespace().map(String::from).collect(),
        author_name: an.into(),
        author_email: ae.into(),
        author_time: at.parse().unwrap_or(0),
        refs: parse_refs(refs),
        subject: subject.into(),
        graph: GraphRow::default(),
        file_path: None,
    })
}

/// Parses `%D` output produced with `--decorate=full`.
fn parse_refs(text: &str) -> Vec<RefLabel> {
    let mut refs = Vec::new();
    for part in text.split(", ").filter(|p| !p.is_empty()) {
        let (name, current) = match part.strip_prefix("HEAD -> ") {
            Some(target) => (target, true),
            None => (part, false),
        };
        let name = name.strip_prefix("tag: ").unwrap_or(name);
        let label = if name == "HEAD" {
            RefLabel {
                name: "HEAD".into(),
                kind: RefKind::Head,
                current: true,
            }
        } else if let Some(n) = name.strip_prefix("refs/heads/") {
            RefLabel {
                name: n.into(),
                kind: RefKind::Branch,
                current,
            }
        } else if let Some(n) = name.strip_prefix("refs/remotes/") {
            if n.ends_with("/HEAD") {
                continue;
            }
            RefLabel {
                name: n.into(),
                kind: RefKind::Remote,
                current: false,
            }
        } else if let Some(n) = name.strip_prefix("refs/tags/") {
            RefLabel {
                name: n.into(),
                kind: RefKind::Tag,
                current: false,
            }
        } else {
            continue;
        };
        refs.push(label);
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decorations() {
        let refs = parse_refs(
            "HEAD -> refs/heads/main, refs/remotes/origin/main, refs/remotes/origin/HEAD, tag: refs/tags/v1",
        );
        assert_eq!(refs.len(), 3);
        assert!(refs[0].current && refs[0].kind == RefKind::Branch && refs[0].name == "main");
        assert_eq!(refs[1].kind, RefKind::Remote);
        assert_eq!((refs[2].kind, refs[2].name.as_str()), (RefKind::Tag, "v1"));
    }
}
