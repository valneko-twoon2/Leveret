//! Commands for the settings screen: the global git settings, the clone folder
//! and the app's folders. App settings are deliberately not agent tools: agents
//! must not change their own access level or the user's identity.

use std::path::PathBuf;

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::blocking;
use crate::error::{AppError, Result};
use crate::git::config::GitSettings;
use crate::{crash, difftool, git, settings};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    version: &'static str,
    git: GitSettings,
    /// The folder new clones go into (the saved one, or `Documents\GitHub`).
    clone_dir: String,
    config_dir: String,
    logs_dir: String,
    diff_tool: bool,
}

/// `Documents\GitHub`, or the folder chosen in the settings.
pub fn clone_dir(app: &AppHandle) -> Result<String> {
    if let Some(dir) = settings::load(&settings::config_dir()).clone_dir
        && git::config::usable_folder(&PathBuf::from(&dir))
    {
        return Ok(dir);
    }
    use tauri::Manager;
    let docs = app.path().document_dir().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(docs.join("GitHub").display().to_string())
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<AppSettings> {
    blocking(move || {
        Ok(AppSettings {
            version: env!("CARGO_PKG_VERSION"),
            git: git::config::get()?,
            clone_dir: clone_dir(&app)?,
            config_dir: settings::config_dir().display().to_string(),
            logs_dir: crash::logs_dir().map(|d| d.display().to_string()).unwrap_or_default(),
            diff_tool: difftool::find().is_some(),
        })
    })
    .await
}

/// Sets one of the editable global git settings (an empty value unsets it).
#[tauri::command]
pub async fn set_git_config(key: String, value: String) -> Result<()> {
    git::config::validate(&key, &value)?;
    blocking(move || git::config::set(&key, &value)).await
}

/// Saves the folder for new clones; `None` goes back to `Documents\GitHub`.
#[tauri::command]
pub async fn set_clone_dir(dir: Option<String>) -> Result<()> {
    if let Some(d) = &dir
        && (d.chars().any(char::is_control) || !git::config::usable_folder(&PathBuf::from(d)))
    {
        return Err(AppError::InvalidInput(d.clone()));
    }
    blocking(move || {
        let config = settings::config_dir();
        let mut s = settings::load(&config);
        s.clone_dir = dir;
        settings::save(&config, &s)
    })
    .await
}

/// Opens one of the app's folders (settings or logs) in Explorer.
#[tauri::command]
pub async fn open_app_folder(app: AppHandle, which: String) -> Result<()> {
    let dir = match which.as_str() {
        "config" => Some(settings::config_dir()),
        "logs" => crash::logs_dir(),
        _ => None,
    }
    .ok_or_else(|| AppError::InvalidInput(which.clone()))?;
    blocking(move || {
        std::fs::create_dir_all(&dir)?;
        app.opener()
            .open_path(dir.display().to_string(), None::<&str>)
            .map_err(|e| AppError::Other(e.to_string()))
    })
    .await
}
