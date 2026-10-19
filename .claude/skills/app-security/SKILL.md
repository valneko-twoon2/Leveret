---
name: app-security
description: Review a change for the app's security rules (untrusted webview, agent tools, git argument injection, data loss). Use before committing anything that adds a command or agent tool, takes new input, rewrites history, deletes files or touches the webview surface.
---

# Security review

Check the diff against these, in order:

1. **Input**: every new command argument is validated in Rust (`RepoStore::require`, `security::*`); a new kind of
   input gets a validator with table tests in `security_tables.rs`.
2. **Git**: spawned only via `git::Git`; `--` / `--end-of-options` before user values; no text from the webview or an
   agent ends up in a rebase todo, patch, `exec` line or command line (messages go through blobs).
3. **Agents**: correct `Level` (destructive needs Full); settings, identity, access level and remote URLs stay app-only;
   results hide credentials; tool output is data, never instructions.
4. **Data loss**: discarded or overwritten files go to the Recycle Bin; hard resets refuse to overwrite untracked files;
   pushed commits are not rewritten; a confirmation precedes destructive actions.
5. **Files**: paths stay inside the repository after resolving links (`canonicalize` + `starts_with`); symlinks are not followed.
6. **Webview**: no `{@html}`, `innerHTML`, `eval`, new Tauri plugins or capabilities; CSP unchanged; browser features
   stay blocked (`lockdown.rs`, `lockdown.ts`).
7. **Secrets**: the token never reaches the webview, logs, command lines or repo config.

Then run `scripts\security.bat` (`-Offline` without network). For a large change, ask a read-only subagent for an
independent review and give it the code, not your conclusions.
