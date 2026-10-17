use std::path::Path;

use serde::Serialize;

use super::status::ChangeKind;
use super::{Git, decode, has_head};
use crate::error::Result;

const FIELD: char = '\x1f';
const RECORD: char = '\x1e';

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogScope {
    /// History of HEAD only.
    Head,
    /// All local branches, remote branches and tags.
    All,
}

pub fn get(repo: &Path, scope: LogScope, limit: u32) -> Result<Vec<LogEntry>> {
    if !has_head(repo)? {
        return Ok(Vec::new());
    }
    let format = format!("--format={RECORD}%H{FIELD}%P{FIELD}%an{FIELD}%ae{FIELD}%at{FIELD}%D{FIELD}%s");
    let mut git = Git::new(repo)
        .args(["log", "--date-order", "--decorate=full", "--no-color"])
        .arg(format)
        .arg(format!("--max-count={limit}"));
    git = match scope {
        LogScope::Head => git.arg("HEAD"),
        LogScope::All => git.args(["HEAD", "--branches", "--remotes", "--tags"]),
    };
    let text = decode(&git.arg("--").run()?.stdout);

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
    let [hash, parents, an, ae, at, refs, subject] = f[..] else { return None };
    Some(LogEntry {
        hash: hash.into(),
        parents: parents.split_whitespace().map(String::from).collect(),
        author_name: an.into(),
        author_email: ae.into(),
        author_time: at.parse().unwrap_or(0),
        refs: parse_refs(refs),
        subject: subject.into(),
        graph: GraphRow::default(),
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
            RefLabel { name: "HEAD".into(), kind: RefKind::Head, current: true }
        } else if let Some(n) = name.strip_prefix("refs/heads/") {
            RefLabel { name: n.into(), kind: RefKind::Branch, current }
        } else if let Some(n) = name.strip_prefix("refs/remotes/") {
            if n.ends_with("/HEAD") {
                continue;
            }
            RefLabel { name: n.into(), kind: RefKind::Remote, current: false }
        } else if let Some(n) = name.strip_prefix("refs/tags/") {
            RefLabel { name: n.into(), kind: RefKind::Tag, current: false }
        } else {
            continue;
        };
        refs.push(label);
    }
    refs
}

/// Assigns each commit a column and the line segments needed to draw the graph.
///
/// `lanes[i]` holds the commit hash that lane `i` is waiting for (a parent of an
/// already placed commit) together with the lane's color. Lane slots are reused
/// once they end so the graph stays narrow.
fn build_graph(entries: &mut [LogEntry]) {
    let mut lanes: Vec<Option<(String, u16)>> = Vec::new();
    let mut next_color: u16 = 0;
    let mut new_color = || {
        let c = next_color;
        next_color = next_color.wrapping_add(1);
        c
    };
    let free_slot = |lanes: &mut Vec<Option<(String, u16)>>| -> usize {
        match lanes.iter().position(Option::is_none) {
            Some(i) => i,
            None => {
                lanes.push(None);
                lanes.len() - 1
            }
        }
    };

    for entry in entries.iter_mut() {
        let mut edges = Vec::new();
        let waiting = |l: &Option<(String, u16)>| l.as_ref().is_some_and(|(h, _)| *h == entry.hash);

        let existing = lanes.iter().position(waiting);
        let col = match existing {
            Some(i) => i,
            None => {
                let i = free_slot(&mut lanes);
                lanes[i] = Some((entry.hash.clone(), new_color()));
                i
            }
        };
        let color = lanes[col].as_ref().map_or(0, |(_, c)| *c);

        // Lines entering from above: lanes merging into this commit and lanes passing through.
        let mut through = Vec::new();
        for (i, lane) in lanes.iter().enumerate() {
            let Some((hash, c)) = lane else { continue };
            if *hash == entry.hash {
                if existing.is_some() {
                    edges.push(Edge { from: i as u16, to: col as u16, kind: EdgeKind::Up, color: *c });
                }
            } else {
                through.push((i, *c));
                edges.push(Edge { from: i as u16, to: i as u16, kind: EdgeKind::Up, color: *c });
            }
        }
        for lane in lanes.iter_mut() {
            if waiting(lane) {
                *lane = None;
            }
        }

        // Lines leaving downwards towards the parents. The first parent always
        // continues this commit's lane (even if another lane already waits for
        // it; the lanes converge at the parent), which keeps the mainline straight.
        for (k, parent) in entry.parents.iter().enumerate() {
            let target = if k == 0 {
                None
            } else {
                lanes.iter().position(|l| l.as_ref().is_some_and(|(h, _)| h == parent))
            };
            let (j, c) = match target {
                Some(j) => (j, lanes[j].as_ref().map_or(0, |(_, c)| *c)),
                None => {
                    let j = if k == 0 { col } else { free_slot(&mut lanes) };
                    let c = if k == 0 { color } else { new_color() };
                    lanes[j] = Some((parent.clone(), c));
                    (j, c)
                }
            };
            edges.push(Edge { from: col as u16, to: j as u16, kind: EdgeKind::Down, color: c });
        }
        for (i, c) in through {
            edges.push(Edge { from: i as u16, to: i as u16, kind: EdgeKind::Down, color: c });
        }

        let width = edges.iter().map(|e| e.from.max(e.to)).chain([col as u16]).max().unwrap_or(0) + 1;
        entry.graph = GraphRow { col: col as u16, color, width, edges };

        while matches!(lanes.last(), Some(None)) {
            lanes.pop();
        }
    }
}

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
    let text = Git::new(repo).args(["show", "-s", "--no-color"]).arg(format).arg(hash).arg("--").run_string()?;
    let f: Vec<&str> = text.splitn(9, FIELD).collect();
    let [h, parents, an, ae, at, cn, ce, ct, body] = f[..] else {
        return Err(crate::error::AppError::Other(format!("unexpected git show output for {hash}")));
    };
    let parents: Vec<String> = parents.split_whitespace().map(String::from).collect();

    // Files changed relative to the first parent (or everything, for a root commit).
    let mut git = Git::new(repo).args(["diff-tree", "-r", "-M", "--name-status", "-z", "--no-commit-id"]);
    git = match parents.first() {
        Some(p) => git.args([p.as_str(), h]),
        None => git.args(["--root", h]),
    };
    let files = parse_name_status(&decode(&git.run()?.stdout));

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
fn parse_name_status(text: &str) -> Vec<CommitFile> {
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

    fn entry(hash: &str, parents: &[&str]) -> LogEntry {
        LogEntry {
            hash: hash.into(),
            parents: parents.iter().map(|p| p.to_string()).collect(),
            author_name: String::new(),
            author_email: String::new(),
            author_time: 0,
            refs: Vec::new(),
            subject: String::new(),
            graph: GraphRow::default(),
        }
    }

    #[test]
    fn graph_for_merge() {
        // m merges a (first parent) and b; both come from root r.
        let mut e = vec![entry("m", &["a", "b"]), entry("b", &["r"]), entry("a", &["r"]), entry("r", &[])];
        build_graph(&mut e);
        assert_eq!(e[0].graph.col, 0);
        assert_eq!(e[1].graph.col, 1);
        assert_eq!(e[2].graph.col, 0);
        assert_eq!(e[3].graph.col, 0);
        // r receives lines from both lanes.
        let ups: Vec<_> = e[3].graph.edges.iter().filter(|x| x.kind == EdgeKind::Up).map(|x| x.from).collect();
        assert_eq!(ups, vec![0, 1]);
        assert_eq!(e[0].graph.width, 2);
    }

    #[test]
    fn parses_decorations() {
        let refs = parse_refs("HEAD -> refs/heads/main, refs/remotes/origin/main, refs/remotes/origin/HEAD, tag: refs/tags/v1");
        assert_eq!(refs.len(), 3);
        assert!(refs[0].current && refs[0].kind == RefKind::Branch && refs[0].name == "main");
        assert_eq!(refs[1].kind, RefKind::Remote);
        assert_eq!((refs[2].kind, refs[2].name.as_str()), (RefKind::Tag, "v1"));
    }

    #[test]
    fn parses_name_status() {
        let f = parse_name_status("M\0a.txt\0R100\0old.txt\0new.txt\0A\0日本語.txt\0");
        assert_eq!(f.len(), 3);
        assert_eq!(f[1].path, "new.txt");
        assert_eq!(f[1].orig_path.as_deref(), Some("old.txt"));
        assert_eq!(f[2].kind, ChangeKind::Added);
    }
}
