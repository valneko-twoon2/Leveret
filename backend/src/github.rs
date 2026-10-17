//! GitHub sign-in (OAuth device flow or a personal access token), account info
//! and repository listing.
//!
//! The token is kept in Windows Credential Manager and never sent to the
//! webview. Git gets it through environment variables (see [`git_auth_env`]),
//! so it never appears on a command line or in a repository's config.

use std::io::Read as _;
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, Result};

const API: &str = "https://api.github.com";
const KEYRING_SERVICE: &str = "Leveret";
const KEYRING_USER: &str = "github.com";
const SCOPES: &str = "repo read:user";
const MAX_REPO_PAGES: usize = 10;

/// OAuth App client id: `LEVERET_GITHUB_CLIENT_ID` at run time, or the value baked in at build time.
pub fn client_id() -> Option<String> {
    std::env::var("LEVERET_GITHUB_CLIENT_ID")
        .ok()
        .or_else(|| option_env!("LEVERET_GITHUB_CLIENT_ID").map(String::from))
        .filter(|id| !id.trim().is_empty())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub login: String,
    pub name: Option<String>,
    pub html_url: String,
    /// Small avatar as a `data:` URL, so the webview needs no network access.
    pub avatar: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCode {
    #[serde(alias = "device_code")]
    pub device_code: String,
    #[serde(alias = "user_code")]
    pub user_code: String,
    #[serde(alias = "verification_uri")]
    pub verification_uri: String,
    #[serde(alias = "expires_in")]
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PollResult {
    Pending,
    SlowDown { interval: u64 },
    Denied,
    Expired,
    Success { account: Account },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubRepo {
    pub full_name: String,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub private: bool,
    pub fork: bool,
    pub clone_url: String,
    pub html_url: String,
    pub pushed_at: Option<String>,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("Leveret/", env!("CARGO_PKG_VERSION")))
        .build()
}

fn http_error(e: ureq::Error) -> AppError {
    match e {
        ureq::Error::Status(401, _) => AppError::GithubUnauthorized,
        ureq::Error::Status(code, resp) => {
            let message = resp
                .into_json::<Value>()
                .ok()
                .and_then(|v| v.get("message").and_then(Value::as_str).map(String::from))
                .unwrap_or_default();
            AppError::GithubApi(format!("HTTP {code} {message}").trim().to_string())
        }
        ureq::Error::Transport(t) => AppError::Network(t.to_string()),
    }
}

fn json<T: serde::de::DeserializeOwned>(resp: ureq::Response) -> Result<T> {
    resp.into_json::<T>().map_err(|e| AppError::GithubApi(e.to_string()))
}

// --- token storage ---------------------------------------------------------

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| AppError::Credential(e.to_string()))
}

fn load_token() -> Result<Option<String>> {
    match entry()?.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Credential(e.to_string())),
    }
}

fn save_token(token: &str) -> Result<()> {
    entry()?
        .set_password(token)
        .map_err(|e| AppError::Credential(e.to_string()))
}

fn delete_token() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Credential(e.to_string())),
    }
}

fn require_token() -> Result<String> {
    load_token()?.ok_or(AppError::GithubNotSignedIn)
}

// --- API ---------------------------------------------------------------------

fn api_get(token: &str, path_and_query: &str) -> Result<ureq::Response> {
    agent()
        .get(&format!("{API}{path_and_query}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(http_error)
}

fn fetch_account(token: &str) -> Result<Account> {
    let user: Value = json(api_get(token, "/user")?)?;
    let field = |k: &str| user.get(k).and_then(Value::as_str).map(String::from);
    let login = field("login").ok_or_else(|| AppError::GithubApi("missing login".into()))?;
    let avatar = field("avatar_url").and_then(|url| fetch_avatar(&url));
    Ok(Account {
        login,
        name: field("name"),
        html_url: field("html_url").unwrap_or_default(),
        avatar,
    })
}

/// Downloads a small avatar and returns it as a data URL (best effort).
fn fetch_avatar(url: &str) -> Option<String> {
    if !url.starts_with("https://avatars.githubusercontent.com/") {
        return None;
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    let resp = agent().get(&format!("{url}{sep}s=64")).call().ok()?;
    let mime = match resp.content_type() {
        m @ ("image/png" | "image/jpeg" | "image/gif" | "image/webp") => m.to_string(),
        _ => return None,
    };
    let mut bytes = Vec::new();
    resp.into_reader().take(512 * 1024).read_to_end(&mut bytes).ok()?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// The signed-in account, or `None`. A token GitHub no longer accepts is discarded.
pub fn account() -> Result<Option<Account>> {
    let Some(token) = load_token()? else { return Ok(None) };
    match fetch_account(&token) {
        Ok(account) => Ok(Some(account)),
        Err(AppError::GithubUnauthorized) => {
            delete_token()?;
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

pub fn sign_in_with_token(token: &str) -> Result<Account> {
    let token = token.trim();
    if token.is_empty() || token.len() > 512 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(AppError::InvalidInput("token".into()));
    }
    let account = fetch_account(token)?;
    save_token(token)?;
    Ok(account)
}

pub fn sign_out() -> Result<()> {
    delete_token()
}

/// Step 1 of the device flow: get a code for the user to enter on github.com.
pub fn start_device_flow() -> Result<DeviceCode> {
    let id = client_id().ok_or(AppError::GithubClientIdMissing)?;
    let resp = agent()
        .post("https://github.com/login/device/code")
        .set("Accept", "application/json")
        .send_form(&[("client_id", id.as_str()), ("scope", SCOPES)])
        .map_err(http_error)?;
    let v: Value = json(resp)?;
    if let Some(err) = v.get("error").and_then(Value::as_str) {
        let desc = v.get("error_description").and_then(Value::as_str).unwrap_or(err);
        return Err(AppError::GithubAuthFailed(desc.to_string()));
    }
    serde_json::from_value(v).map_err(|e| AppError::GithubApi(e.to_string()))
}

/// Step 2 of the device flow: called every `interval` seconds until it is not `Pending`.
pub fn poll_device_flow(device_code: &str) -> Result<PollResult> {
    let id = client_id().ok_or(AppError::GithubClientIdMissing)?;
    let resp = agent()
        .post("https://github.com/login/oauth/access_token")
        .set("Accept", "application/json")
        .send_form(&[
            ("client_id", id.as_str()),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .map_err(http_error)?;
    let v: Value = json(resp)?;
    if let Some(token) = v.get("access_token").and_then(Value::as_str) {
        let account = fetch_account(token)?;
        save_token(token)?;
        return Ok(PollResult::Success { account });
    }
    match v.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => Ok(PollResult::Pending),
        Some("slow_down") => Ok(PollResult::SlowDown {
            interval: v.get("interval").and_then(Value::as_u64).unwrap_or(10),
        }),
        Some("expired_token") => Ok(PollResult::Expired),
        Some("access_denied") => Ok(PollResult::Denied),
        Some(other) => {
            let desc = v.get("error_description").and_then(Value::as_str).unwrap_or(other);
            Err(AppError::GithubAuthFailed(desc.to_string()))
        }
        None => Err(AppError::GithubApi("unexpected token response".into())),
    }
}

/// Repositories the user owns, collaborates on or can access through organizations,
/// most recently pushed first.
pub fn list_repos() -> Result<Vec<GithubRepo>> {
    let token = require_token()?;
    let mut repos = Vec::new();
    for page in 1..=MAX_REPO_PAGES {
        let path = format!(
            "/user/repos?per_page=100&page={page}&sort=pushed&affiliation=owner,collaborator,organization_member"
        );
        let items: Vec<Value> = json(api_get(&token, &path)?)?;
        let count = items.len();
        repos.extend(items.iter().filter_map(parse_repo));
        if count < 100 {
            break;
        }
    }
    Ok(repos)
}

fn parse_repo(v: &Value) -> Option<GithubRepo> {
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(String::from);
    Some(GithubRepo {
        full_name: s("full_name")?,
        owner: v.get("owner")?.get("login")?.as_str()?.to_string(),
        name: s("name")?,
        description: s("description"),
        private: v.get("private").and_then(Value::as_bool).unwrap_or(false),
        fork: v.get("fork").and_then(Value::as_bool).unwrap_or(false),
        clone_url: s("clone_url")?,
        html_url: s("html_url")?,
        pushed_at: s("pushed_at"),
    })
}

/// Environment variables that let git authenticate to github.com over HTTPS
/// with the stored token. Empty for other hosts or when signed out.
pub fn git_auth_env(url: &str) -> Vec<(String, String)> {
    if !url.starts_with("https://github.com/") {
        return Vec::new();
    }
    let Ok(Some(token)) = load_token() else {
        return Vec::new();
    };
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
    vec![
        ("GIT_CONFIG_COUNT".into(), "1".into()),
        ("GIT_CONFIG_KEY_0".into(), "http.https://github.com/.extraheader".into()),
        ("GIT_CONFIG_VALUE_0".into(), format!("AUTHORIZATION: basic {basic}")),
    ]
}

/// [`git_auth_env`] for fetch / pull / push in `repo`: the token is added when a
/// remote is on github.com (the header is scoped to that URL, so other remotes never see it).
pub fn git_auth_env_for_repo(repo: &std::path::Path) -> Vec<(String, String)> {
    crate::git::remote::remote_urls(repo)
        .unwrap_or_default()
        .iter()
        .find(|u| u.starts_with("https://github.com/"))
        .map(|u| git_auth_env(u))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repo_json() {
        let v: Value = serde_json::from_str(
            r#"{"full_name":"octo/leveret","name":"leveret","owner":{"login":"octo"},"description":null,
                "private":true,"fork":false,"clone_url":"https://github.com/octo/leveret.git",
                "html_url":"https://github.com/octo/leveret","pushed_at":"2026-10-17T00:00:00Z"}"#,
        )
        .unwrap();
        let r = parse_repo(&v).unwrap();
        assert_eq!(r.owner, "octo");
        assert!(r.private);
        assert_eq!(r.clone_url, "https://github.com/octo/leveret.git");
    }

    #[test]
    fn parses_device_code() {
        let d: DeviceCode = serde_json::from_str(
            r#"{"device_code":"dc","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":899,"interval":5}"#,
        )
        .unwrap();
        assert_eq!(d.user_code, "ABCD-1234");
        assert_eq!(d.interval, 5);
    }

    #[test]
    fn no_auth_env_for_other_hosts() {
        assert!(git_auth_env("https://gitlab.com/a/b.git").is_empty());
        assert!(git_auth_env("git@github.com:a/b.git").is_empty());
    }
}
