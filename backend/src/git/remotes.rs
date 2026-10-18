//! Managing the configured remotes: listing, adding, renaming, removing and changing URLs.

use std::path::Path;

use serde::Serialize;

use super::{Git, decode, remote};
use crate::error::{AppError, Result};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    pub name: String,
    pub fetch_url: String,
    /// Only set when pushes go to a different URL.
    pub push_url: Option<String>,
}

/// The remotes with their URLs, in the order `git remote` lists them.
pub fn list(repo: &Path) -> Result<Vec<Remote>> {
    let out = Git::new(repo)
        .args(["config", "--null", "--get-regexp", r"^remote\..*\.(url|pushurl)$"])
        .ok_codes(&[0, 1])
        .run()?;
    let urls = parse_urls(&decode(&out.stdout));
    Ok(remote::remotes(repo)?
        .into_iter()
        .map(|name| {
            let find = |kind: &str| {
                urls.iter()
                    .find(|(n, k, _)| *n == name && k == kind)
                    .map(|(_, _, u)| u.clone())
            };
            Remote {
                fetch_url: find("url").unwrap_or_default(),
                push_url: find("pushurl"),
                name,
            }
        })
        .collect())
}

/// `(remote, "url" | "pushurl", value)` from `config --null` records (`key\nvalue\0`).
/// Remote names may contain dots, so the kind is split off the end.
fn parse_urls(text: &str) -> Vec<(String, String, String)> {
    text.split('\0')
        .filter_map(|record| {
            let (key, value) = record.split_once('\n')?;
            let (name, kind) = key.strip_prefix("remote.")?.rsplit_once('.')?;
            Some((name.to_string(), kind.to_string(), value.to_string()))
        })
        .collect()
}

fn exists(repo: &Path, name: &str) -> Result<bool> {
    Ok(remote::remotes(repo)?.iter().any(|r| r == name))
}

fn require(repo: &Path, name: &str) -> Result<()> {
    if exists(repo, name)? {
        Ok(())
    } else {
        Err(AppError::InvalidInput(name.into()))
    }
}

pub fn add(repo: &Path, name: &str, url: &str) -> Result<()> {
    if exists(repo, name)? {
        return Err(AppError::RemoteExists(name.into()));
    }
    Git::new(repo)
        .args(["remote", "add", "--", name, url])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Renames a remote; git also renames its remote-tracking branches and updates the
/// upstreams that point at it.
pub fn rename(repo: &Path, old: &str, new: &str) -> Result<()> {
    require(repo, old)?;
    if old != new && exists(repo, new)? {
        return Err(AppError::RemoteExists(new.into()));
    }
    Git::new(repo)
        .args(["remote", "rename", "--", old, new])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Removes a remote together with its remote-tracking branches.
pub fn remove(repo: &Path, name: &str) -> Result<()> {
    require(repo, name)?;
    Git::new(repo)
        .args(["remote", "remove", "--", name])
        .run()
        .map_err(AppError::classify_git)?;
    Ok(())
}

/// Sets the fetch URL (or, with `push`, the push URL) of a remote.
pub fn set_url(repo: &Path, name: &str, url: &str, push: bool) -> Result<()> {
    require(repo, name)?;
    let mut git = Git::new(repo).args(["remote", "set-url"]);
    if push {
        git = git.arg("--push");
    }
    git.args(["--", name, url]).run().map_err(AppError::classify_git)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{git, git_out, repo_with_commit as repo};

    #[test]
    fn parses_urls_with_dotted_names() {
        let text =
            "remote.origin.url\nhttps://x/y.git\0remote.my.fork.pushurl\nssh://z/w\0remote.my.fork.url\nhttps://a/b\0";
        let p = parse_urls(text);
        assert_eq!(p.len(), 3);
        assert_eq!(p[1], ("my.fork".into(), "pushurl".into(), "ssh://z/w".into()));
    }

    #[test]
    fn add_rename_set_url_remove() {
        let dir = repo();
        let p = dir.path();
        assert!(list(p).unwrap().is_empty());
        add(p, "origin", "https://example.com/a.git").unwrap();
        add(p, "up.stream", "https://example.com/b.git").unwrap();
        assert!(matches!(
            add(p, "origin", "https://example.com/c.git"),
            Err(AppError::RemoteExists(_))
        ));

        set_url(p, "origin", "https://example.com/new.git", false).unwrap();
        set_url(p, "origin", "ssh://git@example.com/push.git", true).unwrap();
        let all = list(p).unwrap();
        assert_eq!(all[0].name, "origin");
        assert_eq!(all[0].fetch_url, "https://example.com/new.git");
        assert_eq!(all[0].push_url.as_deref(), Some("ssh://git@example.com/push.git"));
        assert_eq!(all[1].name, "up.stream");
        assert_eq!(all[1].push_url, None);

        // Remote-tracking branches and upstreams follow a rename and go with a removal.
        git(p, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        git(p, &["config", "branch.main.remote", "origin"]);
        git(p, &["config", "branch.main.merge", "refs/heads/main"]);
        rename(p, "origin", "main-remote").unwrap();
        assert_eq!(git_out(p, &["config", "branch.main.remote"]), "main-remote");
        assert!(git_out(p, &["for-each-ref", "refs/remotes/main-remote"]).contains("refs/remotes/main-remote/main"));
        assert!(matches!(
            rename(p, "main-remote", "up.stream"),
            Err(AppError::RemoteExists(_))
        ));
        assert!(matches!(rename(p, "nope", "x"), Err(AppError::InvalidInput(_))));

        remove(p, "main-remote").unwrap();
        assert_eq!(list(p).unwrap().len(), 1);
        assert!(git_out(p, &["for-each-ref", "refs/remotes"]).is_empty());
        assert!(matches!(remove(p, "main-remote"), Err(AppError::InvalidInput(_))));
    }
}
