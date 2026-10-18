//! Who last changed each line of a file (`git blame`), for the annotate view.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use super::diff::{MAX_LINES, trim_cr};
use super::{Git, decode};
use crate::error::Result;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameCommit {
    /// All zeros for lines that are not committed yet.
    pub hash: String,
    pub author_name: String,
    /// Unix seconds.
    pub author_time: i64,
    pub summary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameLine {
    /// Index into `Blame::commits`.
    pub commit: u32,
    pub text: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Blame {
    pub commits: Vec<BlameCommit>,
    /// The file's lines in order (line `i` is number `i + 1`).
    pub lines: Vec<BlameLine>,
    pub binary: bool,
    pub truncated: bool,
}

/// Blames `path` as it is in `rev`, or in the working tree without one.
/// `ignore_whitespace` (`-w`) skips commits that only changed whitespace.
pub fn blame(repo: &Path, rev: Option<&str>, path: &str, ignore_whitespace: bool) -> Result<Blame> {
    let mut git = Git::new(repo).args(["blame", "--porcelain", "-M"]);
    if ignore_whitespace {
        git = git.arg("-w");
    }
    if let Some(rev) = rev {
        git = git.arg(rev);
    }
    let out = git.arg("--").arg(path).run()?;
    Ok(parse(&out.stdout))
}

/// Parses `--porcelain` output: a `<hash> <orig> <final> [<count>]` header per
/// line, commit details the first time a commit appears, then a TAB and the line.
fn parse(bytes: &[u8]) -> Blame {
    let mut blame = Blame::default();
    let mut index: HashMap<String, u32> = HashMap::new();
    let mut current: Option<u32> = None;
    for line in bytes.split(|&b| b == b'\n') {
        if let Some(content) = line.strip_prefix(b"\t") {
            let Some(commit) = current else { continue };
            if content.contains(&0) {
                return Blame {
                    binary: true,
                    ..Default::default()
                };
            }
            if blame.lines.len() >= MAX_LINES {
                blame.truncated = true;
                break;
            }
            blame.lines.push(BlameLine {
                commit,
                text: decode(trim_cr(content)),
            });
            continue;
        }
        let text = decode(line);
        let mut parts = text.splitn(2, ' ');
        let (key, value) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
        let is_header = key.len() >= 40 && key.bytes().all(|b| b.is_ascii_hexdigit()) && value.contains(' ');
        if is_header {
            let next = blame.commits.len() as u32;
            let i = *index.entry(key.to_string()).or_insert(next);
            if i == next {
                blame.commits.push(BlameCommit {
                    hash: key.to_string(),
                    author_name: String::new(),
                    author_time: 0,
                    summary: String::new(),
                });
            }
            current = Some(i);
            continue;
        }
        let Some(c) = current.and_then(|i| blame.commits.get_mut(i as usize)) else {
            continue;
        };
        match key {
            "author" => c.author_name = value.to_string(),
            "author-time" => c.author_time = value.parse().unwrap_or(0),
            "summary" => c.summary = value.to_string(),
            _ => {}
        }
    }
    blame
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, repo};

    #[test]
    fn parses_porcelain_output() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let text = format!(
            "{a} 1 1 2\nauthor Ken Ito\nauthor-time 100\nsummary first\nfilename f\n\tone\r\n\
             {a} 2 2\n\ttwo\n\
             {b} 3 3 1\nauthor 佐藤\nauthor-time 200\nsummary 二つ目\nprevious {a} f\nfilename f\n\tthree\n"
        );
        let blame = parse(text.as_bytes());
        assert_eq!(blame.commits.len(), 2);
        assert_eq!(blame.commits[1].author_name, "佐藤");
        assert_eq!(blame.commits[0].author_time, 100);
        let lines: Vec<(u32, &str)> = blame.lines.iter().map(|l| (l.commit, l.text.as_str())).collect();
        assert_eq!(lines, [(0, "one"), (0, "two"), (1, "three")]);
        assert!(parse(format!("{a} 1 1 1\nsummary x\n\tbin\0ary\n").as_bytes()).binary);
    }

    #[test]
    fn blames_commits_and_the_working_tree() {
        let dir = repo();
        let p = dir.path();
        std::fs::write(p.join("f.txt"), "one\ntwo\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "最初"]);
        std::fs::write(p.join("f.txt"), "one\nTWO\n").unwrap();
        git(p, &["commit", "-qam", "second"]);
        std::fs::write(p.join("f.txt"), "one\nTWO\nthree\n").unwrap();

        let b = blame(p, None, "f.txt", false).unwrap();
        let summaries: Vec<&str> = b
            .lines
            .iter()
            .map(|l| b.commits[l.commit as usize].summary.as_str())
            .collect();
        assert_eq!(summaries[..2], ["最初", "second"]);
        // Not committed yet.
        assert!(b.commits[b.lines[2].commit as usize].hash.bytes().all(|c| c == b'0'));

        let first = crate::testutil::git_out(p, &["rev-parse", "HEAD~1"]);
        let b = blame(p, Some(&first), "f.txt", false).unwrap();
        assert_eq!(b.lines.len(), 2);
        assert_eq!(b.lines[1].text, "two");
    }
}
