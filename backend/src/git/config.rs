//! The user's global git settings shown in Leveret's settings: identity, default
//! branch name, pull behaviour and line endings. Only these keys can be changed.

use std::path::Path;

use serde::Serialize;

use super::{Git, decode};
use crate::error::{AppError, Result};
use crate::security;

/// The global settings Leveret lets the user edit.
pub const KEYS: [&str; 5] = [
    "user.name",
    "user.email",
    "init.defaultBranch",
    "pull.rebase",
    "core.autocrlf",
];

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitSettings {
    /// `git --version`, e.g. "2.47.1.windows.2".
    pub version: String,
    pub user_name: String,
    pub user_email: String,
    pub default_branch: String,
    pub pull_rebase: String,
    pub autocrlf: String,
}

/// Reads the global settings (empty strings for unset ones).
pub fn get() -> Result<GitSettings> {
    get_in(None)
}

/// `--global`, or `--file <file>` (tests use their own file).
fn scope(file: Option<&Path>) -> Vec<std::ffi::OsString> {
    match file {
        Some(f) => vec!["--file".into(), f.into()],
        None => vec!["--global".into()],
    }
}

fn get_in(file: Option<&Path>) -> Result<GitSettings> {
    let here = std::env::temp_dir();
    let version = decode(&Git::new(&here).arg("--version").run()?.stdout);
    let out = Git::new(&here)
        .arg("config")
        .args(scope(file))
        .args(["--null", "--get-regexp"])
        .arg(r"^(user\.name|user\.email|init\.defaultbranch|pull\.rebase|core\.autocrlf)$")
        .ok_codes(&[0, 1])
        .run()?;
    let mut s = GitSettings {
        version: version.trim().trim_start_matches("git version ").to_string(),
        ..Default::default()
    };
    for entry in decode(&out.stdout).split('\0').filter(|e| !e.is_empty()) {
        let (key, value) = entry.split_once('\n').unwrap_or((entry, ""));
        let slot = match key.to_ascii_lowercase().as_str() {
            "user.name" => &mut s.user_name,
            "user.email" => &mut s.user_email,
            "init.defaultbranch" => &mut s.default_branch,
            "pull.rebase" => &mut s.pull_rebase,
            "core.autocrlf" => &mut s.autocrlf,
            _ => continue,
        };
        *slot = value.to_string();
    }
    Ok(s)
}

/// Checks a value for one of [`KEYS`]; an empty value unsets the key.
pub fn validate(key: &str, value: &str) -> Result<()> {
    let ok = match key {
        _ if value.is_empty() => KEYS.contains(&key),
        "user.name" | "user.email" => {
            value.chars().count() <= 256 && !value.chars().any(char::is_control) && value.trim() == value
        }
        "init.defaultBranch" => security::branch_name(value).is_ok(),
        "pull.rebase" => matches!(value, "true" | "false"),
        "core.autocrlf" => matches!(value, "true" | "input" | "false"),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(AppError::InvalidInput(format!("{key}: {value:?}")))
    }
}

/// Sets (or with an empty value, unsets) a global setting.
pub fn set(key: &str, value: &str) -> Result<()> {
    set_in(None, key, value)
}

fn set_in(file: Option<&Path>, key: &str, value: &str) -> Result<()> {
    validate(key, value)?;
    let here = std::env::temp_dir();
    let git = Git::new(&here).arg("config").args(scope(file));
    if value.is_empty() {
        // Exit code 5: the key wasn't set.
        git.args(["--unset-all", "--", key]).ok_codes(&[0, 5]).run()?;
    } else {
        git.args(["--", key, value]).run()?;
    }
    Ok(())
}

/// Whether `dir` can be used as the folder new clones go into.
pub fn usable_folder(dir: &Path) -> bool {
    dir.is_absolute() && dir.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_values() {
        for (key, value) in [
            ("user.name", "山田 太郎"),
            ("user.email", "taro@example.com"),
            ("init.defaultBranch", "main"),
            ("pull.rebase", "true"),
            ("core.autocrlf", "input"),
            ("user.name", ""),
        ] {
            assert!(validate(key, value).is_ok(), "{key}={value:?}");
        }
        for (key, value) in [
            ("user.name", "a\nb"),
            ("user.name", " padded "),
            ("user.email", &"x".repeat(257)),
            ("init.defaultBranch", "bad name"),
            ("init.defaultBranch", "-x"),
            ("pull.rebase", "merges"),
            ("core.autocrlf", "yes"),
            ("core.editor", "evil"),
            ("alias.x", ""),
            ("include.path", "C:/x"),
        ] {
            assert!(validate(key, value).is_err(), "{key}={value:?}");
        }
    }

    #[test]
    fn reads_and_writes_a_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("gitconfig");
        let f = Some(file.as_path());
        set_in(f, "user.name", "Leveret Test").unwrap();
        set_in(f, "pull.rebase", "true").unwrap();
        set_in(f, "init.defaultBranch", "trunk").unwrap();
        let s = get_in(f).unwrap();
        assert!(!s.version.is_empty());
        assert_eq!((s.user_name.as_str(), s.pull_rebase.as_str()), ("Leveret Test", "true"));
        assert_eq!(s.default_branch, "trunk");
        set_in(f, "pull.rebase", "").unwrap();
        // Unsetting twice is fine.
        set_in(f, "pull.rebase", "").unwrap();
        assert_eq!(get_in(f).unwrap().pull_rebase, "");
        assert!(set_in(f, "core.editor", "evil").is_err());
    }
}
