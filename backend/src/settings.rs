//! App settings and the config directory shared by the app and `leveret --mcp`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::agent::Access;
use crate::error::Result;

/// Same as `identifier` in tauri.conf.json, so both modes use one folder.
const IDENTIFIER: &str = "com.leveret.desktop";

/// `%APPDATA%\com.leveret.desktop` (what Tauri's `app_config_dir` resolves to), or
/// `LEVERET_CONFIG_DIR` when set (benchmarks use their own repository list).
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("LEVERET_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(IDENTIFIER)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// What AI agents may do through the MCP server or in-app agents.
    pub agent_access: Access,
    /// Folder new clones go into; `Documents\GitHub` when unset.
    pub clone_dir: Option<String>,
}

fn file(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

/// Loads the settings; missing or unreadable files give the defaults.
pub fn load(dir: &Path) -> Settings {
    std::fs::read(file(dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, settings: &Settings) -> Result<()> {
    let json = serde_json::to_vec_pretty(settings).map_err(|e| crate::error::AppError::Other(e.to_string()))?;
    write_atomic(&file(dir), &json)
}

/// Writes through a temporary file and a rename, so readers (including another
/// Leveret process) never see a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).agent_access, Access::Standard);
        save(
            dir.path(),
            &Settings {
                agent_access: Access::Full,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(load(dir.path()).agent_access, Access::Full);
        std::fs::write(file(dir.path()), "not json").unwrap();
        assert_eq!(load(dir.path()), Settings::default());
    }
}
