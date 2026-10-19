---
name: add-command
description: Checklist for adding a feature that touches git, the UI and the agent tools (a new Tauri command, agent tool, error code or dialog). Use when adding or changing any user-facing repository action.
---

# Adding a command, end to end

Contract tests (`backend/src/contract_tests.rs`, `agent/tools/mod.rs`) fail until every layer agrees, so do them all:

1. **Git logic** in `backend/src/git/<area>.rs` through `git::Git`; unit tests plus a temporary-repository test.
2. **Validation** in `security.rs` (new validator + table test) for any new kind of input; never accept scripts, patches or rebase todo text from the webview.
3. **Tauri command** in `backend/src/commands/`, registered in `lib.rs`; run long work in `blocking()`.
4. **Agent tool** in `backend/src/agent/tools/<area>.rs` with the right `Level`, added to `ORDER` in `tools/mod.rs`.
   App-only (not a tool): anything that changes settings, identity, access level or remote URLs.
5. **Errors**: new `AppError` variant plus `errors.<code>` in `locales/ja/errors.ts`.
6. **Frontend**: wrapper in `frontend/src/lib/*Api.ts`, action in the store through `app.run()`, UI text in `locales/ja/<area>.ts` only.
7. **Mock**: matching command in `frontend/src/lib/mock/` and the tool in `AGENT_TOOLS`.
8. **Scene and docs**: capture scene in `frontend/src/lib/scenes/`, gallery entries in `docs/screenshots*.md`, the README feature tables and the agent tool count (README.md and README.ja.md).
9. **Checks**: `scripts\test.bat`; for git, parsing, lists or diffs also `scripts\benchmark.bat`. Commit as its own change (short bullets, no trailers).

Pitfalls seen before:
- `app.run()` returns `undefined` on failure. A callback that returns nothing looks like a failure: return a value (`true`) and check `=== undefined`.
- Respect the 500 line / 32 KB limit; split the file as part of the change.
