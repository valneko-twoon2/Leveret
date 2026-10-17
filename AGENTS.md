# AGENTS.md

Guidance for coding agents (and humans) working in this repository.

## Project

Leveret is a simple, GitHub Desktop-style Git client for Windows.
Users register repositories in a list and commit / browse history from there.

- `backend/`: Rust + Tauri 2. Talks to git by running the user's `git.exe` (`src/git/`).
- `frontend/`: Svelte 5 + TypeScript + Vite. All state and actions live in `src/lib/state.svelte.ts`.
- `scripts/`: PowerShell scripts with `.bat` wrappers (see README).
- Each side keeps its own config files (`frontend/package.json`, `backend/Cargo.toml`, `backend/tauri.conf.json`, ...).
  Do not add config files at the repository root unless they are repo-wide (`.gitignore`, `.editorconfig`, ...).

## Language rules

- **English** for code comments, commit messages, PR titles/descriptions, issues, `AGENTS.md`, and `README.md`.
- Japanese appears only in: UI translations (`frontend/src/lib/i18n/locales/ja.ts`), `README.ja.md`,
  and test / mock data that deliberately exercises non-ASCII text.
- A `commit-msg` hook (`.githooks/commit-msg`, enabled by `scripts\setup.ps1`) rejects non-ASCII commit messages.
- `README.md` (English, the base) and `README.ja.md` are kept in sync. No emoji in documentation.

## Commits

- Short imperative summary, at most 72 characters, optionally prefixed with a Conventional Commits type:
  `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `build:`, `chore:`.
- Wrap the body at 72 characters and explain *why* when it is not obvious.
- One logical change per commit. Run `scripts\test.bat` before committing.

## Conventions

- **UI text is never hard-coded.** Add keys to `locales/ja.ts` (it defines the message shape; other locales
  are type-checked against it) and use `t('key')`.
- **Backend errors** are `AppError` variants serialized as `{ code, detail }` (`backend/src/error.rs`).
  When adding a variant, add a matching `errors.<code>` message.
- **Git access** goes through `git::Git` (`backend/src/git/mod.rs`), which sets safe defaults
  (no console window, literal pathspecs, no prompts). Prefer `-z` / porcelain output and parse it in Rust.
- Keep the mock backend (`frontend/src/lib/mock.ts`) in sync with new commands so the UI preview and
  screenshots keep working.
- **New screens or states** get a capture scene in `frontend/src/lib/capture.ts`; check them with
  `scripts\screenshot.bat` (add `-Browser` to skip the Rust build).
- Rust: `cargo fmt` style (`backend/rustfmt.toml`), no clippy warnings. Frontend: `npm run check` must be clean.
- Tests: parsing logic gets unit tests; git behaviour gets tests against a temporary repository
  (see `backend/src/git/integration_tests.rs`).

## Security

The webview is treated as untrusted; the Rust backend is the trust boundary.

- **Validate every command argument** in `backend/src/lib.rs` before it reaches the file system or git:
  repositories must be registered (`RepoStore::require`), paths go through `security::relative_path`,
  object ids through `security::object_id`, clone URLs through `security::clone_url`, folder names through
  `security::folder_name`. Add a validator (with tests in `security.rs`) for any new kind of input.
- **Git** is only spawned via `git::Git`, which disables risky transports (`protocol.ext.allow=never`),
  prompts and pathspec magic. Put `--` before user-supplied paths/URLs.
- **Secrets** (the GitHub token) live in Windows Credential Manager (`github.rs`). Never send them to the
  webview, log them, put them on a command line, or write them to a repository config; git receives them
  through `GIT_CONFIG_*` environment variables only.
- **Webview surface**: keep the CSP strict, `freezePrototype` on, and capabilities limited to the allowlist in
  `scripts/security.ps1`. Network access happens in Rust, never from the webview. External pages open in the
  user's browser through `open_github_url` (github.com only).
- **Browser features stay blocked** (devtools, reload, print, find, zoom, context menu, navigation, new
  windows, downloads): `backend/src/lockdown.rs` (WebView2 settings) and `frontend/src/lib/lockdown.ts`.
  Debug builds can enable devtools with `LEVERET_DEVTOOLS=1`.
- No `{@html}`, `innerHTML`, `eval` or powerful Tauri plugins (shell, fs, http) in the frontend.
- Run `scripts\security.bat` (static rules, `npm audit`, `cargo audit`, security tests) before releases;
  `scripts\test.bat` runs the static rules on every check.

## Commands

| Task | Command |
|---|---|
| Setup (deps + git hooks) | `scripts\setup.ps1` |
| Run (hot reload) | `scripts\dev.bat` (`-Mock` for mock data) |
| UI only, in a browser | `scripts\preview.bat` |
| All checks | `scripts\test.bat` |
| Security check (with dependency audits) | `scripts\security.bat` |
| Screenshots | `scripts\screenshot.bat [-Browser] [-Theme both]` |
| Release build to `artifacts\` | `scripts\publish.bat` |
| Bump version | `scripts\version.bat <x.y.z \| major \| minor \| patch>` |
