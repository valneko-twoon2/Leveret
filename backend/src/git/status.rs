use std::path::Path;

use serde::Serialize;

use super::{Git, decode, git_path_exists};
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Conflicted,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// Original path for renames and copies.
    pub orig_path: Option<String>,
    pub kind: ChangeKind,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    /// Current branch name, `None` when HEAD is detached.
    pub branch: Option<String>,
    /// `None` before the first commit.
    pub head_oid: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub merging: bool,
    pub files: Vec<FileChange>,
}

pub fn get(repo: &Path) -> Result<RepoStatus> {
    let out = Git::new(repo)
        .args(["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=all"])
        .run()?;
    let mut status = parse(&decode(&out.stdout));
    status.merging = git_path_exists(repo, "MERGE_HEAD")?;
    Ok(status)
}

fn parse(text: &str) -> RepoStatus {
    let mut status = RepoStatus::default();
    let mut tokens = text.split('\0').filter(|t| !t.is_empty());

    while let Some(tok) = tokens.next() {
        if let Some(header) = tok.strip_prefix("# ") {
            parse_header(header, &mut status);
            continue;
        }
        match tok.as_bytes()[0] {
            b'1' => {
                // 1 XY sub mH mI mW hH hI <path>
                let f: Vec<&str> = tok.splitn(9, ' ').collect();
                if let [_, xy, .., path] = f[..] {
                    status.files.push(FileChange { path: path.into(), orig_path: None, kind: kind_of(xy) });
                }
            }
            b'2' => {
                // 2 XY sub mH mI mW hH hI Xscore <path>\0<origPath>
                let f: Vec<&str> = tok.splitn(10, ' ').collect();
                let orig = tokens.next().map(String::from);
                if let [_, xy, .., path] = f[..] {
                    status.files.push(FileChange { path: path.into(), orig_path: orig, kind: kind_of(xy) });
                }
            }
            b'u' => {
                // u XY sub m1 m2 m3 mW h1 h2 h3 <path>
                let f: Vec<&str> = tok.splitn(11, ' ').collect();
                if let Some(path) = f.last() {
                    status.files.push(FileChange { path: (*path).into(), orig_path: None, kind: ChangeKind::Conflicted });
                }
            }
            b'?' => {
                status.files.push(FileChange { path: tok[2..].into(), orig_path: None, kind: ChangeKind::Untracked });
            }
            _ => {}
        }
    }
    status.files.sort_by(|a, b| a.path.cmp(&b.path));
    status
}

fn parse_header(header: &str, status: &mut RepoStatus) {
    let (key, value) = header.split_once(' ').unwrap_or((header, ""));
    match key {
        "branch.oid" if value != "(initial)" => status.head_oid = Some(value.into()),
        "branch.head" if value != "(detached)" => status.branch = Some(value.into()),
        "branch.upstream" => status.upstream = Some(value.into()),
        "branch.ab" => {
            for part in value.split(' ') {
                if let Some(n) = part.strip_prefix('+') {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
        }
        _ => {}
    }
}

/// Collapses the index/worktree status pair into the single state shown in the UI.
fn kind_of(xy: &str) -> ChangeKind {
    let has = |c: char| xy.contains(c);
    if has('R') {
        ChangeKind::Renamed
    } else if has('C') {
        ChangeKind::Copied
    } else if has('D') {
        ChangeKind::Deleted
    } else if has('A') {
        ChangeKind::Added
    } else if has('T') {
        ChangeKind::TypeChanged
    } else {
        ChangeKind::Modified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2() {
        let text = "# branch.oid 185b79\0# branch.head master\0# branch.upstream origin/master\0# branch.ab +2 -1\0\
            1 .M N... 100644 100644 100644 aaa aaa a b.txt\0\
            2 R. N... 100644 100644 100644 bbb bbb R100 c.txt\0b.txt\0\
            ? d/日本語.txt\0";
        let s = parse(text);
        assert_eq!(s.branch.as_deref(), Some("master"));
        assert_eq!(s.head_oid.as_deref(), Some("185b79"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(s.files.len(), 3);
        assert_eq!(s.files[0].path, "a b.txt");
        assert_eq!(s.files[0].kind, ChangeKind::Modified);
        assert_eq!(s.files[1].path, "c.txt");
        assert_eq!(s.files[1].orig_path.as_deref(), Some("b.txt"));
        assert_eq!(s.files[1].kind, ChangeKind::Renamed);
        assert_eq!(s.files[2].kind, ChangeKind::Untracked);
    }

    #[test]
    fn parses_initial_and_detached() {
        let s = parse("# branch.oid (initial)\0# branch.head master\0");
        assert_eq!(s.head_oid, None);
        let s = parse("# branch.oid abc\0# branch.head (detached)\0");
        assert_eq!(s.branch, None);
    }
}
