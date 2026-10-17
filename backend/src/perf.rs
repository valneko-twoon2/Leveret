//! Timing of the work users wait for: git processes, GitHub requests, agent tools
//! and startup.
//!
//! Every sample goes to an in-memory ring buffer, read by the app's performance
//! panel (Ctrl+Shift+D) and by `leveret --bench`. Slow samples are logged, and with
//! `LEVERET_PERF=1` (set by `scripts\dev.bat`) every sample, including the ones the
//! frontend reports, is appended to `%LOCALAPPDATA%\com.leveret.desktop\logs\perf.jsonl`.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Samples kept in memory; older ones are dropped.
const CAPACITY: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// A git process, named `git <subcommand>`.
    Git,
    /// A GitHub request, named `<METHOD> <path>` (never the query or headers).
    Http,
    /// An agent tool call (MCP or in-app).
    Tool,
    /// Process start until a startup milestone.
    Startup,
    // Reported by the frontend (see frontend/src/lib/perf.ts).
    /// A backend command round trip, as seen by the webview.
    Ipc,
    /// A screen transition, until the result is painted.
    View,
    /// A user action such as commit or push, start to finish.
    Action,
    /// A main-thread task longer than 50 ms.
    Longtask,
}

impl Kind {
    /// Duration above which a sample is logged as slow.
    fn slow_ms(self) -> f64 {
        match self {
            Kind::Git => 500.0,
            Kind::Http => 2000.0,
            Kind::Tool => 1000.0,
            Kind::Startup => 2000.0,
            Kind::Ipc => 500.0,
            Kind::View => 200.0,
            Kind::Action => 5000.0,
            Kind::Longtask => 200.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    /// Increasing number, so readers can ask for samples newer than the last one seen.
    #[serde(default)]
    pub seq: u64,
    /// Unix milliseconds when the measured work finished.
    pub time: u64,
    pub kind: Kind,
    pub name: String,
    pub ms: f64,
    pub ok: bool,
    /// `backend` or `ui`.
    #[serde(default)]
    pub source: String,
}

struct Ring {
    samples: VecDeque<Sample>,
    next_seq: u64,
}

static RING: Mutex<Ring> = Mutex::new(Ring {
    samples: VecDeque::new(),
    next_seq: 1,
});
static PROCESS_START: OnceLock<Instant> = OnceLock::new();
static LOG_FILE: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();

/// Remembers when the process started. Call first thing in `main`.
pub fn mark_process_start() {
    PROCESS_START.get_or_init(Instant::now);
}

/// Milliseconds since `mark_process_start`.
pub fn uptime_ms() -> f64 {
    PROCESS_START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// Records a startup milestone, measured from process start.
pub fn startup(name: &str) {
    record(Kind::Startup, name, uptime_ms(), true);
}

/// `perf.jsonl` when `LEVERET_PERF` is set (and not `0`).
fn log_file() -> Option<&'static std::path::Path> {
    LOG_FILE
        .get_or_init(|| {
            let on = std::env::var("LEVERET_PERF").is_ok_and(|v| !v.is_empty() && v != "0");
            on.then(|| crate::crash::logs_dir().map(|d| d.join("perf.jsonl")))
                .flatten()
        })
        .as_deref()
}

pub fn enabled_log() -> bool {
    log_file().is_some()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

pub fn record(kind: Kind, name: &str, ms: f64, ok: bool) {
    push(vec![Sample {
        seq: 0,
        time: now_ms(),
        kind,
        name: name.to_string(),
        ms: (ms * 10.0).round() / 10.0,
        ok,
        source: "backend".into(),
    }]);
}

/// Adds samples (numbering them), logs slow ones and appends them to `perf.jsonl`.
pub fn push(mut samples: Vec<Sample>) {
    if samples.is_empty() {
        return;
    }
    {
        // A panic while holding the lock can't leave the ring inconsistent, so reuse it.
        let mut ring = RING.lock().unwrap_or_else(|e| e.into_inner());
        for s in &mut samples {
            s.seq = ring.next_seq;
            ring.next_seq += 1;
            ring.samples.push_back(s.clone());
        }
        let excess = ring.samples.len().saturating_sub(CAPACITY);
        ring.samples.drain(..excess);
    }
    for s in &samples {
        if s.ms >= s.kind.slow_ms() {
            log::warn!("slow {:?} {} took {} ms ({})", s.kind, s.name, s.ms, s.source);
        }
    }
    if let Some(path) = log_file() {
        let _ = (|| -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let mut lines = Vec::new();
            for s in &samples {
                serde_json::to_writer(&mut lines, s)?;
                lines.push(b'\n');
            }
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?
                .write_all(&lines)
        })();
    }
}

/// Samples with `seq > after`, oldest first.
pub fn since(after: u64) -> Vec<Sample> {
    let ring = RING.lock().unwrap_or_else(|e| e.into_inner());
    ring.samples.iter().filter(|s| s.seq > after).cloned().collect()
}

/// Measures from creation until `finish` (or drop, which counts as a failure).
pub struct Span {
    kind: Kind,
    name: String,
    start: Instant,
    done: bool,
}

impl Span {
    pub fn start(kind: Kind, name: impl Into<String>) -> Self {
        Span {
            kind,
            name: name.into(),
            start: Instant::now(),
            done: false,
        }
    }

    pub fn finish(mut self, ok: bool) {
        self.done = true;
        record(self.kind, &self.name, self.elapsed_ms(), ok);
    }

    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        // Dropped without `finish`: an early return or a panic.
        if !self.done {
            record(self.kind, &self.name, self.elapsed_ms(), false);
        }
    }
}

/// Runs `f`, recording how long it took and whether it succeeded.
pub fn time<T, E>(kind: Kind, name: impl Into<String>, f: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let span = Span::start(kind, name);
    let result = f();
    span.finish(result.is_ok());
    result
}

/// Checks a sample reported by the webview (which is untrusted): known kind,
/// short printable name, sane duration. Returns `None` for anything else.
pub fn sanitize_ui_sample(mut s: Sample) -> Option<Sample> {
    let name_ok = !s.name.is_empty()
        && s.name.len() <= 80
        && s.name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " ._:/-<>()".contains(c));
    let kind_ok = matches!(
        s.kind,
        Kind::Ipc | Kind::View | Kind::Action | Kind::Longtask | Kind::Startup
    );
    if !name_ok || !kind_ok || !s.ms.is_finite() || !(0.0..=3_600_000.0).contains(&s.ms) {
        return None;
    }
    s.ms = (s.ms * 10.0).round() / 10.0;
    s.time = now_ms();
    s.source = "ui".into();
    Some(s)
}

/// `git status`, `git stash push`: the subcommand of a git command line,
/// without options, paths or other arguments (they may contain user data).
pub fn git_name<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> String {
    const WITH_ACTION: [&str; 5] = ["stash", "remote", "worktree", "notes", "submodule"];
    let mut words = args.iter().map(|a| a.as_ref().to_string_lossy());
    let mut name = String::from("git");
    while let Some(arg) = words.next() {
        if arg == "-c" || arg == "-C" {
            words.next();
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        name.push(' ');
        name.push_str(&arg);
        if WITH_ACTION.contains(&arg.as_ref())
            && let Some(action) = words.next().filter(|a| !a.starts_with('-'))
        {
            name.push(' ');
            name.push_str(&action);
        }
        break;
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_names_hide_arguments() {
        assert_eq!(git_name(&["status", "--porcelain=v2", "-z"]), "git status");
        assert_eq!(
            git_name(&["-c", "core.editor=x", "commit", "-m", "secret"]),
            "git commit"
        );
        assert_eq!(git_name(&["stash", "push", "--", "a.txt"]), "git stash push");
        assert_eq!(git_name(&["stash", "--", "a.txt"]), "git stash");
        assert_eq!(git_name(&["--no-pager", "log", "C:\\Users\\me"]), "git log");
        assert_eq!(git_name::<&str>(&[]), "git");
    }

    #[test]
    fn spans_and_ring() {
        let before = since(0).last().map_or(0, |s| s.seq);
        time(Kind::Git, "git unit-test-ok", || Ok::<_, ()>(())).unwrap();
        let _ = time(Kind::Git, "git unit-test-err", || Err::<(), _>(()));
        drop(Span::start(Kind::Tool, "unit-test-dropped"));
        let new: Vec<_> = since(before)
            .into_iter()
            .filter(|s| s.name.contains("unit-test"))
            .collect();
        let find = |n: &str| new.iter().find(|s| s.name.ends_with(n)).map(|s| s.ok);
        assert_eq!(find("unit-test-ok"), Some(true));
        assert_eq!(find("unit-test-err"), Some(false));
        assert_eq!(find("unit-test-dropped"), Some(false));
        assert!(new.windows(2).all(|w| w[0].seq < w[1].seq));
    }

    #[test]
    fn ui_samples_are_checked() {
        let sample = |kind, name: &str, ms| Sample {
            seq: 99,
            time: 0,
            kind,
            name: name.into(),
            ms,
            ok: true,
            source: "backend".into(),
        };
        let ok = sanitize_ui_sample(sample(Kind::View, "repo.open", 12.345)).unwrap();
        assert_eq!((ok.ms, ok.source.as_str()), (12.3, "ui"));
        assert!(sanitize_ui_sample(sample(Kind::Ipc, "get_status", 1.0)).is_some());
        for bad in [
            sample(Kind::Git, "git status", 1.0),
            sample(Kind::View, "", 1.0),
            sample(Kind::View, "a\nb", 1.0),
            sample(Kind::View, &"x".repeat(81), 1.0),
            sample(Kind::View, "x", f64::NAN),
            sample(Kind::View, "x", -1.0),
        ] {
            assert!(sanitize_ui_sample(bad).is_none());
        }
    }
}
