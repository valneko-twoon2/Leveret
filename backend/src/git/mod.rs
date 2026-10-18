//! Thin wrapper around the `git` executable.
//!
//! Leveret drives the user's installed git instead of a library so that the
//! user's config, credential helpers, SSH setup and hooks all behave exactly
//! as they do on the command line.

pub mod blame;
pub mod branch;
pub mod clone;
pub mod commit;
pub mod config;
pub mod diff;
pub mod discard;
pub mod image;
pub mod log;
pub mod operation;
pub mod patch;
pub mod rebase;
pub mod remote;
pub mod remotes;
pub mod reset;
pub mod resolve;
pub mod stash;
pub mod status;
pub mod tag;

#[cfg(test)]
mod integration_tests;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;

use crate::error::{AppError, Result};
use crate::perf;

pub struct Git<'a> {
    repo: &'a Path,
    args: Vec<OsString>,
    envs: Vec<(String, String)>,
    stdin: Option<Vec<u8>>,
    ok_codes: &'static [i32],
}

pub struct GitOutput {
    pub stdout: Vec<u8>,
    pub code: i32,
}

impl<'a> Git<'a> {
    pub fn new(repo: &'a Path) -> Self {
        Git {
            repo,
            args: Vec::new(),
            envs: Vec::new(),
            stdin: None,
            ok_codes: &[0],
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn envs(mut self, envs: impl IntoIterator<Item = (String, String)>) -> Self {
        self.envs.extend(envs);
        self
    }

    pub fn stdin(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.stdin = Some(data.into());
        self
    }

    /// Exit codes treated as success (e.g. `git diff --no-index` exits 1 on differences).
    pub fn ok_codes(mut self, codes: &'static [i32]) -> Self {
        self.ok_codes = codes;
        self
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.current_dir(self.repo)
            // Never let a repository-controlled config turn on risky transports.
            .args([
                "-c",
                "core.quotepath=false",
                "-c",
                "color.ui=never",
                "-c",
                "protocol.ext.allow=never",
            ])
            .args(["-c", "protocol.file.allow=user"])
            .args(&self.args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_LITERAL_PATHSPECS", "1")
            // Never open an editor: git accepts its prepared message instead, and a
            // configured `code --wait` style editor can't block a command forever.
            .env("GIT_EDITOR", ":")
            .env("GIT_SEQUENCE_EDITOR", ":")
            // Untranslated messages, so errors such as conflicts can be recognized.
            .env("LC_ALL", "C")
            // Applied last so a command can override a default when it has to.
            .envs(self.envs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        // Tests don't depend on (or get slowed down by) the machine's git config.
        #[cfg(test)]
        cmd.env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", crate::testutil::empty_config());
        cmd
    }

    fn spawn(cmd: &mut Command) -> Result<std::process::Child> {
        cmd.spawn().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AppError::GitNotFound,
            _ => AppError::Io(e),
        })
    }

    pub fn run(self) -> Result<GitOutput> {
        let span = perf::Span::start(perf::Kind::Git, perf::git_name(&self.args));
        let result = self.run_inner();
        span.finish(result.is_ok());
        result
    }

    fn run_inner(self) -> Result<GitOutput> {
        let mut cmd = self.command();
        cmd.stdin(if self.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        let mut child = Self::spawn(&mut cmd)?;

        // Write stdin on a separate thread so a large stdout can't deadlock us.
        let writer = match (self.stdin, child.stdin.take()) {
            (Some(data), Some(mut pipe)) => Some(std::thread::spawn(move || pipe.write_all(&data))),
            _ => None,
        };

        let output = child.wait_with_output()?;
        if let Some(w) = writer {
            // A broken pipe here just means git didn't need all of stdin.
            let _ = w.join();
        }

        let code = output.status.code().unwrap_or(-1);
        if !self.ok_codes.contains(&code) {
            let stderr = decode(&output.stderr).trim().to_string();
            let detail = if stderr.is_empty() {
                decode(&output.stdout).trim().to_string()
            } else {
                stderr
            };
            return Err(AppError::GitFailed(detail));
        }
        Ok(GitOutput {
            stdout: output.stdout,
            code,
        })
    }

    /// Runs git and reports each stderr line (split on `\r` and `\n`, as progress
    /// output uses carriage returns) to `on_line`.
    pub fn run_with_progress(self, on_line: impl FnMut(&str)) -> Result<()> {
        let span = perf::Span::start(perf::Kind::Git, perf::git_name(&self.args));
        let result = self.run_with_progress_inner(on_line);
        span.finish(result.is_ok());
        result
    }

    fn run_with_progress_inner(self, mut on_line: impl FnMut(&str)) -> Result<()> {
        let mut cmd = self.command();
        cmd.stdin(Stdio::null()).stdout(Stdio::null());
        let mut child = Self::spawn(&mut cmd)?;
        let Some(mut stderr) = child.stderr.take() else {
            return Err(AppError::Other("git stderr is not available".into()));
        };

        let mut tail: Vec<String> = Vec::new();
        let mut pending = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = std::io::Read::read(&mut stderr, &mut buf)?;
            if n == 0 {
                break;
            }
            for &b in &buf[..n] {
                if b == b'\r' || b == b'\n' {
                    if !pending.is_empty() {
                        let line = decode(&pending);
                        on_line(&line);
                        tail.push(line);
                        if tail.len() > 20 {
                            tail.remove(0);
                        }
                        pending.clear();
                    }
                } else {
                    pending.push(b);
                }
            }
        }
        if !pending.is_empty() {
            tail.push(decode(&pending));
        }
        let status = child.wait()?;
        if !status.success() {
            let detail: Vec<&str> = tail.iter().map(String::as_str).filter(|l| !l.contains('%')).collect();
            return Err(AppError::GitFailed(detail.join("\n")));
        }
        Ok(())
    }

    pub fn run_string(self) -> Result<String> {
        Ok(decode(&self.run()?.stdout))
    }
}

/// Decodes bytes from git, falling back to Shift_JIS for non UTF-8 content
/// (still common for files in Japanese Windows projects).
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

/// Returns the absolute top-level directory of the repository containing `path`.
pub fn toplevel(path: &Path) -> Result<String> {
    if !path.is_dir() {
        return Err(AppError::NotARepository(path.display().to_string()));
    }
    let out = Git::new(path)
        .args(["rev-parse", "--show-toplevel"])
        .run_string()
        .map_err(|e| match e {
            AppError::GitFailed(_) => AppError::NotARepository(path.display().to_string()),
            e => e,
        })?;
    let top = out.trim();
    #[cfg(windows)]
    let top = top.replace('/', "\\");
    Ok(top.to_string())
}

/// Whether HEAD points at a commit (false in a freshly initialized repository).
pub fn has_head(repo: &Path) -> Result<bool> {
    let out = Git::new(repo)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .ok_codes(&[0, 1])
        .run()?;
    Ok(out.code == 0)
}

/// Object id of the empty tree, used as the "parent" of root commits.
/// Asked from git so it is correct for both SHA-1 and SHA-256 repositories.
pub fn empty_tree(repo: &Path) -> Result<String> {
    Ok(Git::new(repo)
        .args(["hash-object", "-t", "tree", "--stdin"])
        .stdin(Vec::new())
        .run_string()?
        .trim()
        .to_string())
}

/// Absolute path of the repository's git directory (`.git`, or the worktree's own
/// directory for linked worktrees). Cached per repository: it doesn't change, and
/// every git process costs tens of milliseconds just to start on Windows.
pub fn git_dir(repo: &Path) -> Result<PathBuf> {
    static CACHE: Mutex<BTreeMap<PathBuf, PathBuf>> = Mutex::new(BTreeMap::new());
    let cached = CACHE.lock().unwrap_or_else(|e| e.into_inner()).get(repo).cloned();
    if let Some(dir) = cached.filter(|d| d.is_dir()) {
        return Ok(dir);
    }
    let p = Git::new(repo).args(["rev-parse", "--absolute-git-dir"]).run_string()?;
    let dir = PathBuf::from(p.trim());
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(repo.to_path_buf(), dir.clone());
    Ok(dir)
}

/// The result of a scoped thread; a panic in it becomes an error. Used to run
/// independent git processes in parallel.
pub fn join<T>(handle: std::thread::ScopedJoinHandle<'_, Result<T>>) -> Result<T> {
    handle
        .join()
        .unwrap_or_else(|_| Err(AppError::Other("a git worker thread panicked".into())))
}

/// Writes `paths` NUL-separated to a temporary file for `--pathspec-from-file`,
/// so long selections don't hit the command line length limit.
pub struct PathspecFile {
    file: tempfile::NamedTempFile,
}

impl PathspecFile {
    pub fn new<S: AsRef<str>>(paths: &[S]) -> Result<Self> {
        let mut file = tempfile::NamedTempFile::new()?;
        for p in paths {
            file.write_all(p.as_ref().as_bytes())?;
            file.write_all(b"\0")?;
        }
        file.flush()?;
        Ok(PathspecFile { file })
    }

    /// `--pathspec-from-file=<file>` and `--pathspec-file-nul`.
    pub fn args(&self) -> [OsString; 2] {
        let mut a = OsString::from("--pathspec-from-file=");
        a.push(self.file.path());
        [a, OsString::from("--pathspec-file-nul")]
    }
}

/// Progress of a long-running git command (clone, fetch, push).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Git's phase name, e.g. "Receiving objects".
    pub phase: String,
    /// Overall progress across all phases, 0-100.
    pub percent: u8,
}

/// Weight of each phase in the overall progress: (phase prefix, start, end).
pub type Phases = [(&'static str, u8, u8)];

/// Parses a progress line such as `Receiving objects:  45% (450/1000), 1.2 MiB | 1 MiB/s`.
pub fn parse_progress(line: &str, phases: &Phases) -> Option<Progress> {
    let line = line.trim_start_matches("remote: ").trim();
    let (phase, rest) = line.split_once(':')?;
    let percent: u8 = rest.trim_start().split('%').next()?.trim().parse().ok()?;
    let (start, end) = phases
        .iter()
        .find(|(p, ..)| phase.starts_with(p))
        .map(|&(_, s, e)| (s, e))?;
    let overall = start as u32 + (end - start) as u32 * percent.min(100) as u32 / 100;
    Some(Progress {
        phase: phase.to_string(),
        percent: overall as u8,
    })
}

/// Calls `on_progress` for each parsed progress line that differs from the previous one.
pub fn progress_reporter<'a>(phases: &'a Phases, mut on_progress: impl FnMut(Progress) + 'a) -> impl FnMut(&str) + 'a {
    let mut last: Option<Progress> = None;
    move |line| {
        if let Some(p) = parse_progress(line, phases)
            && last.as_ref() != Some(&p)
        {
            on_progress(p.clone());
            last = Some(p);
        }
    }
}
