//! The list of repositories registered by the user, persisted as JSON in the
//! app config directory.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::git;

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

pub struct RepoStore {
    file: PathBuf,
    data: Mutex<RepoList>,
}

impl RepoStore {
    pub fn load(config_dir: &Path) -> Self {
        let file = config_dir.join("repos.json");
        let data = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        RepoStore { file, data: Mutex::new(data) }
    }

    /// Returns the path if it is a registered repository. Commands only operate
    /// on registered repositories, never on arbitrary paths from the webview.
    pub fn require(&self, path: &str) -> Result<PathBuf> {
        let data = self.data.lock().unwrap();
        data.repos
            .iter()
            .find(|r| same_path(&r.path, path))
            .map(|r| PathBuf::from(&r.path))
            .ok_or_else(|| AppError::RepoNotRegistered(path.to_string()))
    }

    pub fn list(&self) -> RepoList {
        self.data.lock().unwrap().clone()
    }

    pub fn add(&self, path: &Path) -> Result<RepoEntry> {
        let top = git::toplevel(path)?;
        let mut data = self.data.lock().unwrap();
        if data.repos.iter().any(|r| same_path(&r.path, &top)) {
            return Err(AppError::RepoAlreadyAdded(top));
        }
        let name = Path::new(&top)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| top.clone());
        let entry = RepoEntry { path: top, name };
        data.repos.push(entry.clone());
        data.repos.sort_by_key(|r| r.name.to_lowercase());
        data.selected = Some(entry.path.clone());
        self.save(&data)?;
        Ok(entry)
    }

    pub fn remove(&self, path: &str) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        data.repos.retain(|r| !same_path(&r.path, path));
        if data.selected.as_deref().is_some_and(|s| same_path(s, path)) {
            data.selected = None;
        }
        self.save(&data)
    }

    pub fn select(&self, path: Option<String>) -> Result<()> {
        if let Some(p) = &path {
            self.require(p)?;
        }
        let mut data = self.data.lock().unwrap();
        data.selected = path;
        self.save(&data)
    }

    fn save(&self, data: &RepoList) -> Result<()> {
        if let Some(dir) = self.file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(data).map_err(|e| AppError::Other(e.to_string()))?;
        std::fs::write(&self.file, json)?;
        Ok(())
    }
}

fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) { a.eq_ignore_ascii_case(b) } else { a == b }
}
