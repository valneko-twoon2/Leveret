//! Commands for the GitHub account: sign-in (device flow or token), sign-out, the
//! user's repositories, and opening github.com pages in the browser.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::blocking;
use crate::error::{AppError, Result};
use crate::{github, security};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubState {
    account: Option<github::Account>,
    /// Whether the OAuth device flow is available (an OAuth App client id is configured).
    device_flow: bool,
}

#[tauri::command]
pub async fn github_state() -> Result<GithubState> {
    let account = blocking(github::account).await?;
    Ok(GithubState {
        account,
        device_flow: github::client_id().is_some(),
    })
}

#[tauri::command]
pub async fn github_start_sign_in() -> Result<github::DeviceCode> {
    blocking(github::start_device_flow).await
}

#[tauri::command]
pub async fn github_poll_sign_in(device_code: String) -> Result<github::PollResult> {
    if device_code.is_empty() || device_code.len() > 256 || !device_code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(AppError::InvalidInput("device code".into()));
    }
    blocking(move || github::poll_device_flow(&device_code)).await
}

#[tauri::command]
pub async fn github_sign_in_with_token(token: String) -> Result<github::Account> {
    blocking(move || github::sign_in_with_token(&token)).await
}

#[tauri::command]
pub async fn github_sign_out() -> Result<()> {
    blocking(github::sign_out).await
}

#[tauri::command]
pub async fn github_list_repos() -> Result<Vec<github::GithubRepo>> {
    blocking(github::list_repos).await
}

/// Opens a github.com page (sign-in page, repository page) in the default browser.
#[tauri::command]
pub fn open_github_url(app: AppHandle, url: String) -> Result<()> {
    security::github_web_url(&url)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Other(e.to_string()))
}
