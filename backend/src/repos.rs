//! The list of repositories registered by the user, persisted as JSON in the
//! app config directory.
//!
//! The app and `leveret --mcp` (started by AI agents) may run at the same time,
//! so the file is the source of truth: it is re-read whenever it changed on disk
//! and written atomically.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::git;
use crate::settings::write_atomic;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoEntry {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoList {
    pub repos: Vec<RepoEntry>,
    #[serde(default)]
    pub selected: Option<String>,
}

struct Cache {
    list: RepoList,
    /// Modification time of the file the list was read from.
    modified: Option<SystemTime>,
}

pub struct RepoStore {
    file: PathBuf,
    cache: Mutex<Cache>,
}

fn modified(file: &Path) -> Option<SystemTime> {
    std::fs::metadata(file).and_then(|m| m.modified()).ok()
}

impl RepoStore {
    pub fn load(config_dir: &Path) -> Self {
        let file = config_dir.join("repos.json");
        let list = match std::fs::read(&file) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                // Keep the unreadable file for the user instead of overwriting it on the next save.
                log::warn!("repos.json is unreadable ({e}); starting with an empty list");
                let _ = std::fs::rename(&file, file.with_extension("json.bak"));
                RepoList::default()
            }),
            Err(_) => RepoList::default(),
        };
        let modified = modified(&file);
        RepoStore {
            file,
            cache: Mutex::new(Cache { list, modified }),
        }
    }

    /// Locks the list, re-reading the file first if another process changed it.
    /// A panic while holding the lock can't leave the list half-updated (every
    /// change is a single assignment or push), so a poisoned lock is reused.
    fn data(&self) -> MutexGuard<'_, Cache> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let now = modified(&self.file);
        if now != cache.modified {
            // A file that can't be read right now (e.g. mid-rename) keeps the cached list.
            if let Some(list) = std::fs::read(&self.file)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
            {
                cache.list = list;
            }
            cache.modified = now;
        }
        cache
    }

    /// Returns the path if it is a registered repository. Commands only operate
    /// on registered repositories, never on arbitrary paths from the webview.
    pub fn require(&self, path: &str) -> Result<PathBuf> {
        let data = self.data();
        data.list
            .repos
            .iter()
            .find(|r| same_path(&r.path, path))
            .map(|r| PathBuf::from(&r.path))
            .ok_or_else(|| AppError::RepoNotRegistered(path.to_string()))
    }

    pub fn list(&self) -> RepoList {
        self.data().list.clone()
    }

    /// Registers the repository containing `path`. `select` makes it the
    /// selected one (the app does that; agents don't change the user's selection).
    pub fn add(&self, path: &Path, select: bool) -> Result<RepoEntry> {
        let top = git::toplevel(path)?;
        let mut data = self.data();
        if data.list.repos.iter().any(|r| same_path(&r.path, &top)) {
            return Err(AppError::RepoAlreadyAdded(top));
        }
        let name = Path::new(&top)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| top.clone());
        let entry = RepoEntry { path: top, name };
        data.list.repos.push(entry.clone());
        data.list.repos.sort_by_key(|r| r.name.to_lowercase());
        if select {
            data.list.selected = Some(entry.path.clone());
        }
        self.save(&mut data)?;
        Ok(entry)
    }

    pub fn remove(&self, path: &str) -> Result<()> {
        let mut data = self.data();
        data.list.repos.retain(|r| !same_path(&r.path, path));
        if data.list.selected.as_deref().is_some_and(|s| same_path(s, path)) {
            data.list.selected = None;
        }
        self.save(&mut data)
    }

    pub fn select(&self, path: Option<String>) -> Result<()> {
        if let Some(p) = &path {
            self.require(p)?;
        }
        let mut data = self.data();
        data.list.selected = path;
        self.save(&mut data)
    }

    fn save(&self, cache: &mut Cache) -> Result<()> {
        let json = serde_json::to_vec_pretty(&cache.list).map_err(|e| AppError::Other(e.to_string()))?;
        write_atomic(&self.file, &json)?;
        cache.modified = modified(&self.file);
        Ok(())
    }
}

fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(b)
    } else {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git::Git::new(dir.path()).args(["init", "-q"]).run().unwrap();
        dir
    }

    #[test]
    fn sees_changes_made_by_another_process() {
        let config = tempfile::tempdir().unwrap();
        let (a, b) = (repo(), repo());
        let app = RepoStore::load(config.path());
        let agent = RepoStore::load(config.path());

        app.add(a.path(), true).unwrap();
        // Another process (e.g. `leveret --mcp`) adds a repository without selecting it.
        let added = agent.add(b.path(), false).unwrap();
        let list = app.list();
        assert_eq!(list.repos.len(), 2);
        assert!(app.require(&added.path).is_ok());
        assert_ne!(list.selected.as_deref(), Some(added.path.as_str()));
    }

    #[test]
    fn unreadable_file_is_kept_as_backup() {
        let config = tempfile::tempdir().unwrap();
        std::fs::write(config.path().join("repos.json"), "{ broken").unwrap();
        let store = RepoStore::load(config.path());
        assert!(store.list().repos.is_empty());
        assert!(config.path().join("repos.json.bak").exists());
    }

    #[test]
    fn unregistered_repositories_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = RepoStore::load(dir.path());
        assert!(matches!(
            store.require("C:\\Windows"),
            Err(AppError::RepoNotRegistered(_))
        ));
        assert!(store.select(Some("C:\\Windows".into())).is_err());
    }
}
