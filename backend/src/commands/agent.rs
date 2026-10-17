//! Commands for AI agents: the settings page (tools, access level, activity), calling
//! a tool in-process, and watching the activity log for changes made by agents.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::blocking;
use crate::error::{AppError, Result};
use crate::repos::RepoStore;
use crate::{agent, settings};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentTool {
    name: &'static str,
    title: &'static str,
    level: agent::Level,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    /// Path of this executable, for the MCP client configuration (`<exe> --mcp`).
    exe_path: String,
    access: agent::Access,
    tools: Vec<AgentTool>,
    activity: Vec<agent::Activity>,
}

#[tauri::command]
pub async fn agent_info() -> Result<AgentInfo> {
    blocking(|| {
        let dir = settings::config_dir();
        let exe_path = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let tools = agent::tools::all()
            .into_iter()
            .map(|t| AgentTool {
                name: t.name,
                title: t.title,
                level: t.level,
            })
            .collect();
        Ok(AgentInfo {
            exe_path,
            access: settings::load(&dir).agent_access,
            tools,
            activity: agent::recent(&dir, 50),
        })
    })
    .await
}

#[tauri::command]
pub async fn set_agent_access(access: agent::Access) -> Result<()> {
    blocking(move || {
        let dir = settings::config_dir();
        let mut s = settings::load(&dir);
        s.agent_access = access;
        settings::save(&dir, &s)
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCallResult {
    text: String,
    is_error: bool,
}

/// Runs an agent tool in-process (for agents running inside Leveret). Same access
/// checks and activity log as the MCP server.
#[tauri::command]
pub async fn agent_call(app: AppHandle, name: String, arguments: serde_json::Value) -> Result<AgentCallResult> {
    blocking(move || {
        let dir = settings::config_dir();
        let ctx = agent::tools::Context {
            store: &app.state::<RepoStore>(),
            config_dir: &dir,
            client: "Leveret",
        };
        Ok(match agent::tools::call(&ctx, &name, &arguments) {
            Ok(out) => AgentCallResult {
                text: out.into_text(),
                is_error: false,
            },
            Err(agent::tools::CallError::UnknownTool) => {
                return Err(AppError::InvalidInput(format!("unknown tool: {name}")));
            }
            Err(agent::tools::CallError::Denied(m) | agent::tools::CallError::Failed(m)) => AgentCallResult {
                text: m,
                is_error: true,
            },
        })
    })
    .await
}

/// Emits `agent-activity` with new entries whenever an agent (usually `leveret --mcp`
/// in another process) changes something, so the app can refresh and tell the user.
pub(crate) fn watch_agent_activity(app: AppHandle) {
    std::thread::spawn(move || {
        let dir = settings::config_dir();
        let file = agent::activity_file(&dir);
        let modified = |f: &std::path::Path| std::fs::metadata(f).and_then(|m| m.modified()).ok();
        let mut seen = modified(&file);
        let mut last = agent::recent(&dir, 1).last().map_or(0, |a| a.time);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let now = modified(&file);
            if now == seen {
                continue;
            }
            seen = now;
            let new = agent::since(&dir, last);
            if let Some(newest) = new.last() {
                last = newest.time;
                let _ = app.emit("agent-activity", new);
            }
        }
    });
}
