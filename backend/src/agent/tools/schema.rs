//! Building blocks for tool definitions: the JSON schemas of their arguments.

use serde_json::{Value, json};

use super::Tool;
use crate::agent::Level;

/// A tool definition (handlers live next to it, in the same area module).
pub(super) fn tool(
    name: &'static str,
    title: &'static str,
    level: Level,
    open_world: bool,
    description: &'static str,
    input_schema: Value,
) -> Tool {
    Tool {
        name,
        title,
        description,
        level,
        open_world,
        input_schema,
    }
}

pub(super) fn object(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

pub(super) fn repo_prop() -> Value {
    json!({
        "type": "string",
        "description": "Repository: its name or path from list_repositories. A path inside the repository (e.g. your working directory) also works."
    })
}

pub(super) fn string(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

pub(super) fn boolean(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}

pub(super) fn paths(description: &str) -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": description })
}

pub(super) fn integer(description: &str, min: u64, max: u64) -> Value {
    json!({ "type": "integer", "description": description, "minimum": min, "maximum": max })
}

pub(super) fn repo_only() -> Value {
    object(json!({ "repo": repo_prop() }), &["repo"])
}
