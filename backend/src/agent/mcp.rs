//! MCP server over stdio: `leveret --mcp`.
//!
//! An MCP client (Claude Code, Codex, GitHub Copilot, Claude Desktop, ...) starts
//! this process and exchanges newline-delimited JSON-RPC 2.0 messages on
//! stdin/stdout. No window opens and no network port is used, so only the
//! process that started the server can talk to it. Logs never go to stdout.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use super::tools::{self, CallError, Context};
use super::{Access, Level};
use crate::repos::RepoStore;
use crate::settings;

/// Protocol versions this server speaks, newest first.
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

pub struct Server {
    store: RepoStore,
    config_dir: PathBuf,
    /// Client name from `initialize`, used in the activity log.
    client: Mutex<String>,
}

impl Server {
    pub fn new(config_dir: &Path) -> Self {
        Server {
            store: RepoStore::load(config_dir),
            config_dir: config_dir.to_path_buf(),
            client: Mutex::new("MCP client".into()),
        }
    }

    /// Handles one line of input; returns the line to send back, if any.
    pub fn handle(&self, line: &str) -> Option<String> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let response = match serde_json::from_str::<Value>(line) {
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
            // Batches (protocol 2025-03-26) are answered with an array.
            Ok(Value::Array(batch)) => {
                let replies: Vec<Value> = batch.iter().filter_map(|m| self.message(m)).collect();
                (!replies.is_empty()).then_some(Value::Array(replies))
            }
            Ok(message) => self.message(&message),
        };
        response.map(|r| r.to_string())
    }

    fn message(&self, msg: &Value) -> Option<Value> {
        let Some(method) = msg.get("method").and_then(Value::as_str) else {
            // A response to a request we never send, or garbage: nothing to answer.
            return msg.get("id").map(|id| error(id.clone(), -32600, "invalid request"));
        };
        // Notifications (no id) get no response.
        let id = msg.get("id")?.clone();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        Some(match method {
            "initialize" => {
                if let Some(name) = params.pointer("/clientInfo/name").and_then(Value::as_str) {
                    *self.client.lock().unwrap_or_else(|e| e.into_inner()) = name.chars().take(100).collect();
                }
                let asked = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let version = PROTOCOL_VERSIONS
                    .iter()
                    .find(|v| **v == asked)
                    .unwrap_or(&PROTOCOL_VERSIONS[0]);
                result(
                    id,
                    json!({
                        "protocolVersion": version,
                        "capabilities": { "tools": { "listChanged": true } },
                        "serverInfo": { "name": "leveret", "title": "Leveret", "version": env!("CARGO_PKG_VERSION") },
                        "instructions": tools::INSTRUCTIONS,
                    }),
                )
            }
            "ping" => result(id, json!({})),
            "tools/list" => result(id, json!({ "tools": self.list() })),
            "tools/call" => self.call(id, &params),
            _ => error(id, -32601, &format!("method not found: {method}")),
        })
    }

    fn access(&self) -> Access {
        settings::load(&self.config_dir).agent_access
    }

    fn list(&self) -> Vec<Value> {
        tools::allowed(self.access())
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "title": t.title,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                    "annotations": {
                        "title": t.title,
                        "readOnlyHint": t.level == Level::Read,
                        "destructiveHint": t.level == Level::Destructive,
                        "idempotentHint": t.level == Level::Read,
                        "openWorldHint": t.open_world,
                    },
                })
            })
            .collect()
    }

    fn call(&self, id: Value, params: &Value) -> Value {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return error(id, -32602, "missing tool name");
        };
        let args = params.get("arguments").cloned().unwrap_or(Value::Null);
        let client = self.client.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let ctx = Context {
            store: &self.store,
            config_dir: &self.config_dir,
            client: &client,
        };
        match tools::call(&ctx, name, &args) {
            Ok(output) => result(
                id,
                json!({ "content": [{ "type": "text", "text": output.into_text() }], "isError": false }),
            ),
            Err(CallError::UnknownTool) => error(id, -32602, &format!("unknown tool: {name}")),
            Err(CallError::Denied(msg) | CallError::Failed(msg)) => result(
                id,
                json!({ "content": [{ "type": "text", "text": msg }], "isError": true }),
            ),
        }
    }
}

fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn send(out: &Mutex<std::io::Stdout>, line: &str) -> bool {
    let mut out = out.lock().unwrap_or_else(|e| e.into_inner());
    writeln!(out, "{line}").and_then(|()| out.flush()).is_ok()
}

/// Runs the server until stdin closes. Returns the process exit code.
pub fn serve() -> i32 {
    let config_dir = settings::config_dir();
    let server = Server::new(&config_dir);
    let out = Arc::new(Mutex::new(std::io::stdout()));

    // Tell the client when the user changes the access level in the app, so it
    // re-reads the tool list.
    {
        let out = Arc::clone(&out);
        let mut last = settings::load(&config_dir).agent_access;
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(2));
                let now = settings::load(&config_dir).agent_access;
                if now != last {
                    last = now;
                    let note = json!({ "jsonrpc": "2.0", "method": "notifications/tools/list_changed" });
                    if !send(&out, &note.to_string()) {
                        break;
                    }
                }
            }
        });
    }

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(reply) = server.handle(&line)
            && !send(&out, &reply)
        {
            break;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use crate::testutil::{self, git};

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, Server) {
        let config = tempfile::tempdir().unwrap();
        let repo = testutil::repo();
        let p = repo.path();
        std::fs::write(p.join("a.txt"), "one\n").unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-qm", "first"]);
        let server = Server::new(config.path());
        server.store.add(p, false).unwrap();
        (config, repo, server)
    }

    fn request(server: &Server, id: u64, method: &str, params: Value) -> Value {
        let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        serde_json::from_str(&server.handle(&msg.to_string()).unwrap()).unwrap()
    }

    fn call(server: &Server, name: &str, args: Value) -> (bool, String) {
        let r = request(server, 9, "tools/call", json!({ "name": name, "arguments": args }));
        let res = &r["result"];
        (
            res["isError"].as_bool().unwrap(),
            res["content"][0]["text"].as_str().unwrap().to_string(),
        )
    }

    #[test]
    fn initialize_and_list_tools() {
        let (config, _repo, server) = setup();
        let r = request(
            &server,
            1,
            "initialize",
            json!({ "protocolVersion": "2025-03-26", "clientInfo": { "name": "claude-code" } }),
        );
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(r["result"]["serverInfo"]["name"], "leveret");
        assert!(
            server
                .handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_none()
        );

        let tools = request(&server, 2, "tools/list", json!({}));
        let names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(
            names.contains(&"commit") && !names.contains(&"discard_changes"),
            "standard access by default"
        );

        settings::save(
            config.path(),
            &Settings {
                agent_access: Access::ReadOnly,
                ..Default::default()
            },
        )
        .unwrap();
        let tools = request(&server, 3, "tools/list", json!({}));
        assert!(
            tools["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .all(|t| t["annotations"]["readOnlyHint"] == true)
        );

        let r = request(&server, 4, "nope", json!({}));
        assert_eq!(r["error"]["code"], -32601);
        let r: Value = serde_json::from_str(&server.handle("{not json").unwrap()).unwrap();
        assert_eq!(r["error"]["code"], -32700);
    }

    #[test]
    fn tools_change_the_repository_and_are_logged() {
        let (config, repo, server) = setup();
        request(
            &server,
            1,
            "initialize",
            json!({ "clientInfo": { "name": "test-agent" } }),
        );
        let name = repo.path().file_name().unwrap().to_str().unwrap().to_string();

        std::fs::write(repo.path().join("a.txt"), "two\n").unwrap();
        std::fs::write(repo.path().join("新規.txt"), "new\n").unwrap();
        let (err, text) = call(&server, "get_status", json!({ "repo": name }));
        assert!(!err, "{text}");
        assert!(text.contains("新規.txt"));
        let (_, diff) = call(
            &server,
            "get_diff",
            json!({ "repo": repo.path().display().to_string() }),
        );
        assert!(
            diff.contains("-one") && diff.contains("+two") && diff.contains("+new"),
            "{diff}"
        );

        // A path inside the repository works too.
        let (err, text) = call(
            &server,
            "commit",
            json!({ "repo": repo.path().join("sub").display().to_string(), "summary": "second" }),
        );
        assert!(!err, "{text}");
        let (_, log) = call(&server, "get_log", json!({ "repo": name, "message": "SECOND" }));
        let log: Value = serde_json::from_str(&log).unwrap();
        assert_eq!(log.as_array().unwrap().len(), 1);
        let (_, detail) = call(&server, "get_commit", json!({ "repo": name, "commit": "HEAD" }));
        assert!(detail.contains("+two"));

        let (err, _) = call(&server, "create_branch", json!({ "repo": name, "name": "feature/x" }));
        assert!(!err);
        let (err, _) = call(&server, "switch_branch", json!({ "repo": name, "branch": "main" }));
        assert!(!err);

        // Destructive tools need full access; the reason is explained to the model.
        let (err, text) = call(&server, "delete_branch", json!({ "repo": name, "name": "feature/x" }));
        assert!(err && text.contains("full"), "{text}");
        settings::save(
            config.path(),
            &Settings {
                agent_access: Access::Full,
                ..Default::default()
            },
        )
        .unwrap();
        let (err, text) = call(&server, "delete_branch", json!({ "repo": name, "name": "feature/x" }));
        assert!(!err, "{text}");

        // Errors carry a hint; unknown repositories list the registered ones.
        let (err, text) = call(&server, "get_status", json!({ "repo": "missing" }));
        assert!(err && text.contains(&name), "{text}");
        let (err, text) = call(&server, "switch_branch", json!({ "repo": name, "branch": "--orphan" }));
        assert!(err, "{text}");

        let log = super::super::recent(config.path(), 10);
        assert!(
            log.iter()
                .any(|a| a.tool == "commit" && a.client == "test-agent" && a.ok)
        );
        assert!(log.iter().all(|a| a.tool != "get_status"), "reads are not logged");
    }

    #[test]
    fn access_off_hides_everything() {
        let (config, _repo, server) = setup();
        settings::save(
            config.path(),
            &Settings {
                agent_access: Access::Off,
                ..Default::default()
            },
        )
        .unwrap();
        let tools = request(&server, 1, "tools/list", json!({}));
        assert!(tools["result"]["tools"].as_array().unwrap().is_empty());
        let (err, _) = call(&server, "list_repositories", json!({}));
        assert!(err);
        let r = request(&server, 2, "tools/call", json!({ "name": "no_such_tool" }));
        assert_eq!(r["error"]["code"], -32602);
    }
}
