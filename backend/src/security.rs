//! Validation of values that arrive from the webview.
//!
//! The frontend is treated as untrusted: every command argument that ends up
//! in a file path or on a git command line goes through one of these checks.

use std::path::{Component, Path};

use crate::error::{AppError, Result};

fn invalid(what: &str, value: &str) -> AppError {
    AppError::InvalidInput(format!("{what}: {value:?}"))
}

fn has_control_chars(s: &str) -> bool {
    s.chars().any(char::is_control)
}

/// A path relative to the repository root that cannot escape it
/// (no absolute paths, drive letters or `..`).
pub fn relative_path(path: &str) -> Result<&str> {
    if path.is_empty() || has_control_chars(path) || path.contains(':') || path.starts_with('-') {
        return Err(invalid("path", path));
    }
    let normal = Path::new(path)
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    if !normal {
        return Err(invalid("path", path));
    }
    Ok(path)
}

/// A full or abbreviated object id. Rejecting anything else also prevents
/// option injection (e.g. a "hash" of `--output=...`).
pub fn object_id(id: &str) -> Result<&str> {
    if (4..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(id)
    } else {
        Err(invalid("object id", id))
    }
}

/// A remote URL we are willing to clone from: https, ssh or scp-like `user@host:path`.
/// Local paths, `file://` and transport helpers such as `ext::` are refused.
pub fn clone_url(url: &str) -> Result<&str> {
    let ok = !url.is_empty()
        && url.len() <= 2048
        && !has_control_chars(url)
        && !url.contains(char::is_whitespace)
        && !url.starts_with('-')
        && !url.contains("::")
        && (url.starts_with("https://") || url.starts_with("ssh://") || is_scp_like(url));
    if ok { Ok(url) } else { Err(invalid("url", url)) }
}

fn is_scp_like(url: &str) -> bool {
    // user@host:path, where host has no slash before the colon.
    match (url.find('@'), url.find(':')) {
        (Some(at), Some(colon)) => at > 0 && colon > at + 1 && !url[..colon].contains('/'),
        _ => false,
    }
}

/// A single folder name for a new clone (no separators, reserved characters or names).
pub fn folder_name(name: &str) -> Result<&str> {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1",
        "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let bad_char = name.chars().any(|c| c.is_control() || r#"\/:*?"<>|"#.contains(c));
    let stem = name.split('.').next().unwrap_or_default();
    if name.is_empty()
        || name.len() > 255
        || bad_char
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || name.ends_with(' ')
        || name.starts_with('-')
        || RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem))
    {
        return Err(invalid("folder name", name));
    }
    Ok(name)
}

/// A branch name git accepts (the rules of `git check-ref-format --branch`),
/// which also can't be mistaken for an option.
pub fn branch_name(name: &str) -> Result<&str> {
    let bad_char = name
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c));
    let bad_part = name
        .split('/')
        .any(|p| p.is_empty() || p.starts_with('.') || p.ends_with(".lock"));
    if name.is_empty()
        || name.len() > 255
        || bad_char
        || bad_part
        || name.starts_with('-')
        || name.ends_with('.')
        || name.contains("..")
        || name.contains("@{")
        || name == "@"
        || name == "HEAD"
    {
        return Err(invalid("branch name", name));
    }
    Ok(name)
}

/// A full local or remote branch ref: `refs/heads/<branch>` or `refs/remotes/<remote>/<branch>`.
pub fn branch_ref(name: &str) -> Result<&str> {
    let ok = match (name.strip_prefix("refs/heads/"), name.strip_prefix("refs/remotes/")) {
        (Some(branch), _) => branch_name(branch).is_ok(),
        (_, Some(remote)) => remote.contains('/') && branch_name(remote).is_ok(),
        _ => false,
    };
    if ok { Ok(name) } else { Err(invalid("branch", name)) }
}

/// A start point for a new branch: an object id or a full branch ref.
pub fn revision(rev: &str) -> Result<&str> {
    if object_id(rev).is_ok() || branch_ref(rev).is_ok() {
        Ok(rev)
    } else {
        Err(invalid("revision", rev))
    }
}

/// A revision expression for `git rev-parse` such as `HEAD~2`, `main^`, `v1.0` or a hash.
/// No options, ranges (`..`), paths (`:`) or whitespace.
pub fn revision_expr(rev: &str) -> Result<&str> {
    let ok_char = |c: char| c.is_alphanumeric() || "._/~^@{}-+".contains(c);
    if rev.is_empty() || rev.len() > 255 || rev.starts_with('-') || rev.contains("..") || !rev.chars().all(ok_char) {
        return Err(invalid("revision", rev));
    }
    Ok(rev)
}

/// Only GitHub pages may be opened in the user's browser from the app.
pub fn github_web_url(url: &str) -> Result<&str> {
    if (url.starts_with("https://github.com/") || url == "https://github.com") && !has_control_chars(url) {
        Ok(url)
    } else {
        Err(invalid("url", url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        assert!(relative_path("src/main.rs").is_ok());
        assert!(relative_path("日本語/ファイル.txt").is_ok());
        assert!(relative_path("a b/c.txt").is_ok());
        for bad in [
            "",
            "../secret",
            "a/../../b",
            "/etc/passwd",
            "C:\\Windows\\win.ini",
            "\\\\server\\share",
            "-x",
            "a\0b",
        ] {
            assert!(relative_path(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn object_ids() {
        assert!(object_id("abc1234").is_ok());
        assert!(object_id(&"a".repeat(40)).is_ok());
        for bad in ["", "abc", "--output=x", "HEAD", "abc123g", &"a".repeat(65)] {
            assert!(object_id(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn clone_urls() {
        for ok in [
            "https://github.com/a/b.git",
            "ssh://git@github.com/a/b.git",
            "git@github.com:a/b.git",
        ] {
            assert!(clone_url(ok).is_ok(), "{ok:?} should be accepted");
        }
        for bad in [
            "",
            "ext::sh -c touch% /tmp/x",
            "file:///C:/repo",
            "C:\\repo",
            "--upload-pack=evil",
            "https://x y",
            "http://github.com/a/b",
            "/local/path:x",
        ] {
            assert!(clone_url(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn folder_names() {
        assert!(folder_name("leveret").is_ok());
        assert!(folder_name("my.repo").is_ok());
        for bad in ["", ".", "..", "a/b", "a\\b", "con", "NUL.txt", "x:", "trail.", "-rf"] {
            assert!(folder_name(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn branch_names() {
        for ok in ["main", "feature/login", "機能/ログイン", "fix-1.2", "user@host"] {
            assert!(branch_name(ok).is_ok(), "{ok:?} should be accepted");
        }
        for bad in [
            "", "-f", "--force", "a b", "a..b", "a~1", "a^", "a:b", "a?", "a*", "a[", "a\\b", "/a", "a/", "a//b",
            ".hidden", "a/.b", "a.lock", "a.", "@", "a@{1}", "HEAD", "a\tb", "a\0b",
        ] {
            assert!(branch_name(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn branch_refs_and_revisions() {
        assert!(branch_ref("refs/heads/main").is_ok());
        assert!(branch_ref("refs/remotes/origin/feature/x").is_ok());
        for bad in [
            "main",
            "refs/tags/v1",
            "refs/remotes/origin",
            "refs/heads/-x",
            "refs/heads/../x",
            "--all",
        ] {
            assert!(branch_ref(bad).is_err(), "{bad:?} should be rejected");
        }
        assert!(revision("abc1234").is_ok());
        assert!(revision("refs/heads/main").is_ok());
        assert!(revision("HEAD~1").is_err());
        assert!(revision("--orphan").is_err());
    }

    #[test]
    fn revision_exprs() {
        for ok in [
            "HEAD",
            "HEAD~2",
            "main^",
            "v1.0",
            "abc1234",
            "origin/feature/x",
            "HEAD@{1}",
            "機能",
        ] {
            assert!(revision_expr(ok).is_ok(), "{ok:?} should be accepted");
        }
        for bad in ["", "-n", "--all", "a..b", "HEAD:file", "a b", "a;b", "$(x)", "a\nb"] {
            assert!(revision_expr(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn github_urls() {
        assert!(github_web_url("https://github.com/login/device").is_ok());
        assert!(github_web_url("https://github.com.evil.com/").is_err());
        assert!(github_web_url("javascript:alert(1)").is_err());
        assert!(github_web_url("file:///C:/Windows").is_err());
    }
}
