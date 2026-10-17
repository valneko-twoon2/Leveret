//! Consistency checks across the backend, the frontend and the docs: every
//! command is registered, called and mocked; every error code has a message;
//! every agent tool is described and mirrored in the mock. One test catches a
//! whole class of "forgot to add it over there" mistakes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::agent::Level;
use crate::agent::tools;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Every `.ts` / `.svelte` file under `frontend/src`.
fn frontend_sources() -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "ts" || x == "svelte") {
                out.push((p.clone(), std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(&repo_root().join("frontend/src"), &mut out);
    out
}

/// The text between quotes right after each occurrence of `prefix`, e.g. the
/// names in `case 'list_tags':` for the prefix `case '`.
fn quoted_after(text: &str, prefix: &str) -> BTreeSet<String> {
    text.match_indices(prefix)
        .filter_map(|(i, _)| {
            let rest = &text[i + prefix.len()..];
            rest.find('\'').map(|end| rest[..end].to_string())
        })
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .collect()
}

/// Commands registered in `generate_handler!` in lib.rs.
fn registered_commands() -> BTreeSet<String> {
    let lib = read("backend/src/lib.rs");
    let start = lib.find("generate_handler![").expect("generate_handler! in lib.rs");
    let end = start + lib[start..].find(']').unwrap();
    lib[start..end]
        .split(',')
        .filter_map(|s| s.trim().rsplit("::").next().map(str::to_string))
        .filter(|s| !s.is_empty() && !s.contains('['))
        .collect()
}

/// Commands the frontend sends through `func<T>('name', ...)` / `func('name')`:
/// `call` (timed, served by the mock in the browser preview) or `invoke`
/// (raw, only for app-only commands such as screenshots).
fn called_commands(func: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (_, text) in frontend_sources() {
        for (i, _) in text.match_indices(func) {
            let rest = &text[i + func.len()..];
            let rest = match rest.strip_prefix('<') {
                // Skip the type argument (may contain `<...>` itself, e.g. `Record<...>`).
                Some(r) => {
                    let mut depth = 1;
                    let end = r.char_indices().find(|&(_, c)| {
                        depth += match c {
                            '<' => 1,
                            '>' => -1,
                            _ => 0,
                        };
                        depth == 0
                    });
                    match end {
                        Some((j, _)) => &r[j + 1..],
                        None => continue,
                    }
                }
                None => rest,
            };
            if let Some(name) = rest.strip_prefix("('").and_then(|r| r.split('\'').next()) {
                out.insert(name.to_string());
            }
        }
    }
    out
}

#[test]
fn every_command_is_registered_called_and_mocked() {
    let registered = registered_commands();
    let via_call = called_commands("call");
    let called: BTreeSet<String> = via_call.union(&called_commands("invoke")).cloned().collect();
    assert!(registered.len() > 50, "parsing lib.rs found only {registered:?}");

    let unregistered: Vec<_> = called.difference(&registered).collect();
    assert!(
        unregistered.is_empty(),
        "called by the frontend but not registered in lib.rs: {unregistered:?}"
    );
    let uncalled: Vec<_> = registered.difference(&called).collect();
    assert!(
        uncalled.is_empty(),
        "registered but never called by the frontend (api.ts): {uncalled:?}"
    );

    let mock: String = frontend_sources()
        .into_iter()
        .filter(|(p, _)| p.components().any(|c| c.as_os_str() == "mock"))
        .map(|(_, t)| t)
        .collect();
    let mocked = quoted_after(&mock, "case '");
    let missing: Vec<_> = via_call.difference(&mocked).collect();
    assert!(
        missing.is_empty(),
        "commands missing from the mock backend (frontend/src/lib/mock): {missing:?}"
    );
}

/// Codes returned by `AppError::code()`.
fn error_codes() -> BTreeSet<String> {
    let src = read("backend/src/error.rs");
    let start = src.find("fn code(&self)").expect("AppError::code");
    let end = start + src[start..].find("fn detail(").expect("AppError::detail");
    src[start..end]
        .lines()
        .filter_map(|l| l.split("=> \"").nth(1).and_then(|r| r.split('"').next()))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_error_code_has_a_message() {
    let codes = error_codes();
    assert!(codes.len() > 20, "parsing error.rs found only {codes:?}");
    let catalog = read("frontend/src/lib/i18n/locales/ja/errors.ts");
    let keys: BTreeSet<String> = catalog
        .lines()
        .filter_map(|l| l.trim().split_once(':').map(|(k, _)| k.to_string()))
        .filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric()))
        .filter(|k| k != "errors" && k != "title")
        .collect();
    let missing: Vec<_> = codes.difference(&keys).collect();
    assert!(
        missing.is_empty(),
        "error codes without an errors.<code> message in ja/errors.ts: {missing:?}"
    );
    let unused: Vec<_> = keys.difference(&codes).collect();
    assert!(
        unused.is_empty(),
        "errors.<key> messages without a backend error code: {unused:?}"
    );
}

#[test]
fn every_agent_tool_is_described_and_mirrored_in_the_mock() {
    let all = tools::all();
    let mut names = BTreeSet::new();
    let mut problems = Vec::new();
    for t in &all {
        if !names.insert(t.name) {
            problems.push(format!("{}: duplicate name", t.name));
        }
        if t.title.is_empty() {
            problems.push(format!("{}: no title", t.name));
        }
        // Models pick tools by their description: say what it does and when to use it.
        if t.description.len() < 40 || !t.description.ends_with('.') {
            problems.push(format!("{}: description too short: {:?}", t.name, t.description));
        }
        if t.input_schema["type"] != "object" {
            problems.push(format!("{}: input schema is not an object", t.name));
        }
        for r in t.input_schema["required"].as_array().into_iter().flatten() {
            let key = r.as_str().unwrap_or_default();
            if t.input_schema["properties"].get(key).is_none() {
                problems.push(format!("{}: required {key} has no schema", t.name));
            }
        }
    }
    assert!(problems.is_empty(), "agent tool problems:\n{}", problems.join("\n"));

    // The mock's list (frontend/src/lib/mock/data.ts) shows the same tools at the same levels.
    let data = read("frontend/src/lib/mock/data.ts");
    let start = data.find("AGENT_TOOLS").expect("AGENT_TOOLS in mock/data.ts");
    let block = &data[start..start + data[start..].find("].map((t)").expect("end of AGENT_TOOLS")];
    for (level, word) in [
        (Level::Read, "read"),
        (Level::Write, "write"),
        (Level::Destructive, "destructive"),
    ] {
        let line = block
            .lines()
            .find(|l| l.contains(&format!("level: '{word}'")))
            .unwrap_or_else(|| panic!("no {word} line in AGENT_TOOLS"));
        let mocked: BTreeSet<String> = quoted_after(line, "'").into_iter().filter(|n| n != word).collect();
        let real: BTreeSet<String> = all
            .iter()
            .filter(|t| t.level == level)
            .map(|t| t.name.to_string())
            .collect();
        assert_eq!(
            mocked, real,
            "AGENT_TOOLS ({word}) in mock/data.ts differs from the backend's tools"
        );
    }

    // The README states the number of tools.
    for (file, label) in [("README.md", "tools**"), ("README.ja.md", "個のツール**")] {
        let readme = read(file);
        let stated = readme
            .find(label)
            .and_then(|i| readme[..i].rsplit("**").next().map(|n| n.trim().to_string()))
            .unwrap_or_default();
        assert_eq!(stated, all.len().to_string(), "{file} states {stated} tools");
    }
}
