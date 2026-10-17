//! Tags: listing, creating and deleting them, and pushing them to a remote.

use std::path::Path;

use serde::Serialize;

use super::{Git, decode, remote};
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub name: String,
    /// The commit the tag points at (peeled through annotated tags).
    pub target: String,
    /// Annotated (with a message and tagger) rather than lightweight.
    pub annotated: bool,
    /// First line of the tag message, or of the commit message for lightweight tags.
    pub subject: String,
    /// Unix seconds: when an annotated tag was made, or the commit date.
    pub time: i64,
}

const FIELD: &str = "%00";

/// All tags, newest first.
pub fn list(repo: &Path) -> Result<Vec<Tag>> {
    let format = [
        "%(refname:strip=2)",
        "%(objecttype)",
        "%(objectname)",
        "%(*objectname)",
        "%(creatordate:unix)",
        "%(contents:subject)",
    ]
    .join(FIELD);
    let out = Git::new(repo)
        .args([
            "for-each-ref",
            "--sort=-creatordate",
            &format!("--format={format}"),
            "refs/tags",
        ])
        .run()?;
    Ok(parse(&decode(&out.stdout)))
}

fn parse(text: &str) -> Vec<Tag> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\0').collect();
            let [name, kind, oid, peeled, time, subject] = f[..] else {
                return None;
            };
            let annotated = kind == "tag";
            Some(Tag {
                name: name.to_string(),
                target: if annotated && !peeled.is_empty() { peeled } else { oid }.to_string(),
                annotated,
                subject: subject.to_string(),
                time: time.parse().unwrap_or(0),
            })
        })
        .collect()
}

fn exists(repo: &Path, name: &str) -> Result<bool> {
    let out = Git::new(repo)
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/tags/{name}"))
        .ok_codes(&[0, 1])
        .run()?;
    Ok(out.code == 0)
}

/// Creates tag `name` at `target` (a commit). With a non-empty `message` the tag
/// is annotated, otherwise lightweight.
pub fn create(repo: &Path, name: &str, target: &str, message: &str) -> Result<()> {
    if exists(repo, name)? {
        return Err(AppError::TagExists(name.into()));
    }
    let mut git = Git::new(repo).arg("tag");
    if !message.trim().is_empty() {
        git = git
            .args(["--annotate", "--cleanup=strip", "--message"])
            .arg(message.trim());
    }
    git.args(["--", name, target]).run().map_err(AppError::classify_git)?;
    Ok(())
}

/// Deletes a local tag.
pub fn delete(repo: &Path, name: &str) -> Result<()> {
    Git::new(repo)
        .args(["tag", "--delete", "--", name])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Pushes a tag to the default remote (`origin`, or the only one).
pub fn push(repo: &Path, envs: Vec<(String, String)>, name: &str) -> Result<()> {
    let remote = remote::default_remote(repo)?;
    Git::new(repo)
        .envs(envs)
        .args(["push", "--", &remote])
        .arg(format!("refs/tags/{name}:refs/tags/{name}"))
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Deletes a tag from the default remote (the local tag is kept).
pub fn delete_remote(repo: &Path, envs: Vec<(String, String)>, name: &str) -> Result<()> {
    let remote = remote::default_remote(repo)?;
    Git::new(repo)
        .envs(envs)
        .args(["push", "--delete", "--", &remote])
        .arg(format!("refs/tags/{name}"))
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        Git::new(dir).args(args).run().unwrap();
    }

    #[test]
    fn parses_lightweight_and_annotated_tags() {
        let text = [
            "v2\0tag\0aaaa\0cccc\0",
            "1700000100\0Release 2\n",
            "v1\0commit\0bbbb\0\0",
            "1700000000\0First commit\n",
        ]
        .concat();
        let text = text.as_str();
        let tags = parse(text);
        assert_eq!(tags.len(), 2);
        assert_eq!(
            (tags[0].name.as_str(), tags[0].target.as_str(), tags[0].annotated),
            ("v2", "cccc", true)
        );
        assert_eq!(
            (tags[1].target.as_str(), tags[1].annotated, tags[1].time),
            ("bbbb", false, 1_700_000_000)
        );
    }

    #[test]
    fn create_list_push_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let remote_dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(remote_dir.path(), &["init", "-q", "--bare"]);
        git(p, &["init", "-q", "-b", "main"]);
        git(p, &["config", "user.name", "Test"]);
        git(p, &["config", "user.email", "test@example.com"]);
        std::fs::write(p.join("a.txt"), "a\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "init"]);
        let head = decode(&Git::new(p).args(["rev-parse", "HEAD"]).run().unwrap().stdout)
            .trim()
            .to_string();

        create(p, "v1.0", &head, "").unwrap();
        create(p, "release/2", &head, "リリース 2\n\n詳細").unwrap();
        assert!(matches!(create(p, "v1.0", &head, ""), Err(AppError::TagExists(_))));
        let tags = list(p).unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.iter().all(|t| t.target == head));
        let annotated = tags.iter().find(|t| t.name == "release/2").unwrap();
        assert!(annotated.annotated);
        assert_eq!(annotated.subject, "リリース 2");

        git(
            p,
            &["remote", "add", "origin", &remote_dir.path().display().to_string()],
        );
        push(p, Vec::new(), "v1.0").unwrap();
        let remote_tags = |dir: &Path| decode(&Git::new(dir).args(["tag", "--list"]).run().unwrap().stdout);
        assert_eq!(remote_tags(remote_dir.path()).trim(), "v1.0");
        delete_remote(p, Vec::new(), "v1.0").unwrap();
        assert_eq!(remote_tags(remote_dir.path()).trim(), "");

        delete(p, "v1.0").unwrap();
        assert_eq!(list(p).unwrap().len(), 1);
    }
}
