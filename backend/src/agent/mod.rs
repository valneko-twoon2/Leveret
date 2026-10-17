//! AI agent integration.
//!
//! Leveret's features are available to AI agents as tools ([`tools`]). The same
//! tools are served over MCP (`leveret --mcp`, see [`mcp`]) to agents running in
//! other apps on this PC (Claude Code, Codex, GitHub Copilot, ...), and can be
//! called in-process by agents running inside Leveret.
//!
//! What agents may do is controlled by [`Access`] (stored in settings.json and
//! checked on every call). Every change an agent makes is recorded in the
//! activity log, which the app shows to the user.

pub mod mcp;
pub mod tools;

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What agents are allowed to do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Access {
    /// No tools at all.
    Off,
    /// Only tools that read.
    ReadOnly,
    /// Everything except destructive tools.
    #[default]
    Standard,
    /// Everything, including tools that can lose work (discard, delete branch, ...).
    Full,
}

/// How much a tool can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Read,
    Write,
    /// Can lose work that isn't recoverable from git alone.
    Destructive,
}

impl Access {
    pub fn allows(self, level: Level) -> bool {
        match level {
            Level::Read => self >= Access::ReadOnly,
            Level::Write => self >= Access::Standard,
            Level::Destructive => self == Access::Full,
        }
    }
}

/// One change made by an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    /// Unix milliseconds.
    pub time: i64,
    /// Agent name as reported by the MCP client (e.g. "claude-code"), or "Leveret".
    pub client: String,
    pub tool: String,
    pub repo: Option<String>,
    pub ok: bool,
    /// Short result or error message.
    pub message: String,
}

const ACTIVITY_FILE: &str = "agent-activity.jsonl";
/// The log is trimmed to `KEEP` entries once it grows past `MAX` lines.
const MAX: usize = 500;
const KEEP: usize = 200;

pub fn activity_file(dir: &Path) -> PathBuf {
    dir.join(ACTIVITY_FILE)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Appends an entry to the activity log. Failures are ignored: the log is
/// informational and must never make a tool call fail.
pub fn record(dir: &Path, activity: &Activity) {
    let Ok(line) = serde_json::to_string(activity) else {
        return;
    };
    let path = activity_file(dir);
    let _ = std::fs::create_dir_all(dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
    }
    let entries = read_all(dir);
    if entries.len() > MAX {
        let text: String = entries[entries.len() - KEEP..]
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .map(|l| l + "\n")
            .collect();
        let _ = crate::settings::write_atomic(&path, text.as_bytes());
    }
}

fn read_all(dir: &Path) -> Vec<Activity> {
    std::fs::read_to_string(activity_file(dir))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// The most recent `limit` entries, newest last.
pub fn recent(dir: &Path, limit: usize) -> Vec<Activity> {
    let all = read_all(dir);
    all[all.len().saturating_sub(limit)..].to_vec()
}

/// Entries recorded after `time` (Unix milliseconds).
pub fn since(dir: &Path, time: i64) -> Vec<Activity> {
    read_all(dir).into_iter().filter(|a| a.time > time).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_levels() {
        assert!(!Access::Off.allows(Level::Read));
        assert!(Access::ReadOnly.allows(Level::Read) && !Access::ReadOnly.allows(Level::Write));
        assert!(Access::Standard.allows(Level::Write) && !Access::Standard.allows(Level::Destructive));
        assert!(Access::Full.allows(Level::Destructive));
    }

    #[test]
    fn activity_log_is_trimmed() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..(MAX + 1) {
            let a = Activity {
                time: i as i64,
                client: "c".into(),
                tool: "t".into(),
                repo: None,
                ok: true,
                message: String::new(),
            };
            record(dir.path(), &a);
        }
        let all = recent(dir.path(), usize::MAX);
        assert_eq!(all.len(), KEEP);
        assert_eq!(all.last().unwrap().time, MAX as i64);
        assert_eq!(since(dir.path(), MAX as i64 - 2).len(), 2);
    }
}
