//! Local issues, stored in `<git common dir>/leveret/issues.json`: inside the
//! repository's git directory, so they move with the repository but are never
//! committed and don't show up as changes. Linked worktrees share the file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::{Comment, Issue, IssueDetail, IssueState, IssueUpdate, Label, Source, StateFilter, now_secs};
use crate::error::{AppError, Result};
use crate::git::Git;
use crate::security;
use crate::settings::write_atomic;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalIssue {
    id: u64,
    title: String,
    body: String,
    state: IssueState,
    #[serde(default)]
    labels: Vec<String>,
    created_at: i64,
    updated_at: i64,
    #[serde(default)]
    comments: Vec<Comment>,
    /// Set once publishing created the GitHub issue; a retry continues from there.
    #[serde(default)]
    published_as: Option<u64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Store {
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    issues: Vec<LocalIssue>,
}

impl LocalIssue {
    fn to_issue(&self) -> Issue {
        Issue {
            source: Source::Local,
            id: self.id,
            title: self.title.clone(),
            body: self.body.clone(),
            state: self.state,
            labels: self
                .labels
                .iter()
                .map(|name| Label {
                    name: name.clone(),
                    color: None,
                })
                .collect(),
            author: None,
            created_at: self.created_at,
            updated_at: self.updated_at,
            comments: self.comments.len() as u32,
            url: None,
            published_as: self.published_as,
        }
    }

    fn detail(&self) -> IssueDetail {
        IssueDetail {
            issue: self.to_issue(),
            comments: self.comments.clone(),
        }
    }
}

/// Serializes read-modify-write cycles within this process (the file lock in
/// [`locked`] does the same across processes).
static LOCK: Mutex<()> = Mutex::new(());

/// `<git common dir>/leveret/issues.json`. Cached per repository (it doesn't change).
fn file(repo: &Path) -> Result<PathBuf> {
    static CACHE: Mutex<BTreeMap<PathBuf, PathBuf>> = Mutex::new(BTreeMap::new());
    if let Some(f) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).get(repo) {
        return Ok(f.clone());
    }
    let dir = Git::new(repo)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .run_string()?;
    let f = PathBuf::from(dir.trim()).join("leveret").join("issues.json");
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(repo.to_path_buf(), f.clone());
    Ok(f)
}

fn load(path: &Path) -> Result<Store> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Other(format!("{} is unreadable: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Store::default()),
        Err(e) => Err(e.into()),
    }
}

fn save(path: &Path, store: &Store) -> Result<()> {
    let json = serde_json::to_vec_pretty(store).map_err(|e| AppError::Other(e.to_string()))?;
    write_atomic(path, &json)
}

/// Runs `f` with the store locked: the in-process mutex plus an OS lock on
/// `issues.lock`, so the app and `leveret --mcp` never interleave their changes.
fn locked<T>(repo: &Path, f: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    let path = file(repo)?;
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.with_file_name("issues.lock"))?;
    // Released when `lock` is dropped.
    lock.lock()?;
    f(&path)
}

/// Runs `f` on the loaded store and saves it afterwards.
fn modify<T>(repo: &Path, f: impl FnOnce(&mut Store) -> Result<T>) -> Result<T> {
    locked(repo, |path| {
        let mut store = load(path)?;
        let result = f(&mut store)?;
        save(path, &store)?;
        Ok(result)
    })
}

fn find(store: &mut Store, id: u64) -> Result<&mut LocalIssue> {
    store
        .issues
        .iter_mut()
        .find(|i| i.id == id)
        .ok_or(AppError::IssueNotFound(id))
}

/// Local issues, newest first.
pub fn list(repo: &Path, filter: StateFilter) -> Result<Vec<Issue>> {
    let store = load(&file(repo)?)?;
    let mut issues: Vec<Issue> = store
        .issues
        .iter()
        .filter(|i| filter.matches(i.state))
        .map(LocalIssue::to_issue)
        .collect();
    issues.sort_by_key(|i| std::cmp::Reverse(i.id));
    Ok(issues)
}

pub fn get(repo: &Path, id: u64) -> Result<IssueDetail> {
    let mut store = load(&file(repo)?)?;
    Ok(find(&mut store, id)?.detail())
}

pub fn create(repo: &Path, title: &str, body: &str) -> Result<Issue> {
    security::issue_title(title)?;
    security::issue_text(body)?;
    modify(repo, |store| {
        let id = store.next_id.max(store.issues.iter().map(|i| i.id).max().unwrap_or(0)) + 1;
        store.next_id = id;
        let now = now_secs();
        let issue = LocalIssue {
            id,
            title: title.trim().to_string(),
            body: body.to_string(),
            state: IssueState::Open,
            labels: Vec::new(),
            created_at: now,
            updated_at: now,
            comments: Vec::new(),
            published_as: None,
        };
        let shown = issue.to_issue();
        store.issues.push(issue);
        Ok(shown)
    })
}

pub fn update(repo: &Path, id: u64, change: &IssueUpdate) -> Result<Issue> {
    change.validate()?;
    modify(repo, |store| {
        let issue = find(store, id)?;
        if let Some(t) = &change.title {
            issue.title = t.trim().to_string();
        }
        if let Some(b) = &change.body {
            issue.body = b.clone();
        }
        if let Some(s) = change.state {
            issue.state = s;
        }
        issue.updated_at = now_secs();
        Ok(issue.to_issue())
    })
}

pub fn comment(repo: &Path, id: u64, body: &str) -> Result<Comment> {
    security::issue_comment(body)?;
    modify(repo, |store| {
        let issue = find(store, id)?;
        let comment = Comment {
            id: issue.comments.iter().map(|c| c.id).max().unwrap_or(0) + 1,
            author: None,
            body: body.to_string(),
            created_at: now_secs(),
        };
        issue.comments.push(comment.clone());
        issue.updated_at = comment.created_at;
        Ok(comment)
    })
}

pub fn delete(repo: &Path, id: u64) -> Result<()> {
    modify(repo, |store| {
        find(store, id)?;
        store.issues.retain(|i| i.id != id);
        Ok(())
    })
}

/// What publishing needs from GitHub (a trait, so tests can fake it).
pub trait Remote {
    fn create(&mut self, title: &str, body: &str) -> Result<Issue>;
    fn get(&mut self, number: u64) -> Result<Issue>;
    fn comment(&mut self, number: u64, body: &str) -> Result<()>;
    fn close(&mut self, number: u64) -> Result<()>;
}

/// Publishes a local issue: creates it on GitHub, posts its comments in order,
/// closes it there if it was closed locally, then removes the local copy and
/// returns the GitHub issue.
///
/// The store stays locked throughout, so the issue can't change meanwhile and two
/// publishes can't race. Progress is saved after every step (the GitHub number
/// right after creating, each comment once posted): if a step fails, the local
/// issue keeps what is left, and publishing again resumes instead of creating a
/// second issue.
pub fn publish(repo: &Path, id: u64, remote: &mut impl Remote) -> Result<Issue> {
    locked(repo, |path| {
        let mut store = load(path)?;
        let mut local = find(&mut store, id)?.clone();
        let mut created = None;
        let number = match local.published_as {
            Some(n) => n,
            None => {
                let issue = remote.create(&local.title, &local.body)?;
                local.published_as = Some(issue.id);
                let number = issue.id;
                created = Some(issue);
                *find(&mut store, id)? = local.clone();
                save(path, &store)?;
                number
            }
        };
        while let Some(c) = local.comments.first().cloned() {
            remote.comment(number, &c.body)?;
            local.comments.remove(0);
            *find(&mut store, id)? = local.clone();
            save(path, &store)?;
        }
        if local.state == IssueState::Closed {
            remote.close(number)?;
        }
        store.issues.retain(|i| i.id != id);
        save(path, &store)?;
        // The issue as it is now (comments, state); fall back to the created one.
        match remote.get(number) {
            Ok(issue) => Ok(issue),
            Err(e) => created.ok_or(e),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        Git::new(dir.path()).args(["init", "-q"]).run().unwrap();
        dir
    }

    #[test]
    fn create_update_comment_delete() {
        let dir = repo();
        let r = dir.path();
        let a = create(r, "  最初の課題 ", "本文").unwrap();
        let b = create(r, "Second", "").unwrap();
        assert_eq!((a.id, b.id, a.title.as_str()), (1, 2, "最初の課題"));
        assert_eq!(
            list(r, StateFilter::All)
                .unwrap()
                .iter()
                .map(|i| i.id)
                .collect::<Vec<_>>(),
            [2, 1]
        );

        let change = IssueUpdate {
            state: Some(IssueState::Closed),
            ..Default::default()
        };
        update(r, 1, &change).unwrap();
        assert_eq!(list(r, StateFilter::Open).unwrap().len(), 1);
        assert_eq!(list(r, StateFilter::Closed).unwrap()[0].id, 1);

        comment(r, 2, "a comment").unwrap();
        let d = get(r, 2).unwrap();
        assert_eq!((d.issue.comments, d.comments[0].body.as_str()), (1, "a comment"));

        delete(r, 2).unwrap();
        assert!(matches!(get(r, 2), Err(AppError::IssueNotFound(2))));
        // Ids are never reused.
        assert_eq!(create(r, "Third", "").unwrap().id, 3);
        // Nothing shows up in the working tree.
        let status = Git::new(r).args(["status", "--porcelain"]).run_string().unwrap();
        assert!(status.is_empty(), "{status}");
    }

    #[test]
    fn invalid_input_is_rejected() {
        let dir = repo();
        assert!(create(dir.path(), "   ", "").is_err());
        assert!(create(dir.path(), "a\u{0}b", "").is_err());
        assert!(comment(dir.path(), 1, "x").is_err());
    }

    /// A fake GitHub: records calls, fails the n-th comment (1-based) if asked.
    #[derive(Default)]
    struct Fake {
        created: Vec<(String, String)>,
        posted: Vec<(u64, String)>,
        closed: Vec<u64>,
        fail_create: bool,
        fail_comment: Option<usize>,
    }

    fn github_issue(number: u64) -> Issue {
        Issue {
            source: Source::Github,
            id: number,
            title: String::new(),
            body: String::new(),
            state: IssueState::Open,
            labels: Vec::new(),
            author: None,
            created_at: 0,
            updated_at: 0,
            comments: 0,
            url: None,
            published_as: None,
        }
    }

    impl Remote for Fake {
        fn create(&mut self, title: &str, body: &str) -> Result<Issue> {
            if self.fail_create {
                return Err(AppError::Network("offline".into()));
            }
            self.created.push((title.into(), body.into()));
            Ok(github_issue(42))
        }
        fn get(&mut self, number: u64) -> Result<Issue> {
            Ok(github_issue(number))
        }
        fn comment(&mut self, number: u64, body: &str) -> Result<()> {
            if self.fail_comment == Some(self.posted.len() + 1) {
                self.fail_comment = None;
                return Err(AppError::Network("offline".into()));
            }
            self.posted.push((number, body.into()));
            Ok(())
        }
        fn close(&mut self, number: u64) -> Result<()> {
            self.closed.push(number);
            Ok(())
        }
    }

    #[test]
    fn publish_moves_the_issue_and_its_comments() {
        let dir = repo();
        let r = dir.path();
        create(r, "Bug", "steps").unwrap();
        comment(r, 1, "first").unwrap();
        comment(r, 1, "second").unwrap();
        update(
            r,
            1,
            &IssueUpdate {
                state: Some(IssueState::Closed),
                ..Default::default()
            },
        )
        .unwrap();
        let mut gh = Fake::default();
        let issue = publish(r, 1, &mut gh).unwrap();
        assert_eq!(issue.id, 42);
        assert_eq!(gh.created, [("Bug".to_string(), "steps".to_string())]);
        assert_eq!(gh.posted, [(42, "first".to_string()), (42, "second".to_string())]);
        assert_eq!(gh.closed, [42]);
        assert!(list(r, StateFilter::All).unwrap().is_empty());
    }

    #[test]
    fn failed_create_keeps_the_local_issue() {
        let dir = repo();
        let r = dir.path();
        create(r, "Bug", "").unwrap();
        let mut gh = Fake {
            fail_create: true,
            ..Default::default()
        };
        assert!(publish(r, 1, &mut gh).is_err());
        let left = list(r, StateFilter::All).unwrap();
        assert_eq!((left.len(), left[0].published_as), (1, None));
    }

    #[test]
    fn failed_comment_resumes_without_a_second_issue() {
        let dir = repo();
        let r = dir.path();
        create(r, "Bug", "").unwrap();
        for c in ["one", "two", "three"] {
            comment(r, 1, c).unwrap();
        }
        let mut gh = Fake {
            fail_comment: Some(2),
            ..Default::default()
        };
        assert!(publish(r, 1, &mut gh).is_err());
        // Created and the first comment posted: both remembered, the rest kept.
        let d = get(r, 1).unwrap();
        assert_eq!(d.issue.published_as, Some(42));
        assert_eq!(
            d.comments.iter().map(|c| c.body.as_str()).collect::<Vec<_>>(),
            ["two", "three"]
        );

        publish(r, 1, &mut gh).unwrap();
        assert_eq!(gh.created.len(), 1);
        let posted: Vec<_> = gh.posted.iter().map(|(_, b)| b.as_str()).collect();
        assert_eq!(posted, ["one", "two", "three"]);
        assert!(list(r, StateFilter::All).unwrap().is_empty());
    }
}
