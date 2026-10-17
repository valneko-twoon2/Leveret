//! Commands for performance measuring: the backend's samples, timings reported by
//! the frontend, and saving the benchmark report.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use super::blocking;
use crate::capture::StartupOptions;
use crate::error::{AppError, Result};
use crate::{perf, settings};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfSnapshot {
    /// Backend samples newer than the requested sequence number.
    samples: Vec<perf::Sample>,
    /// Milliseconds since the process started.
    uptime_ms: f64,
}

#[tauri::command]
pub fn perf_samples(after: u64) -> PerfSnapshot {
    PerfSnapshot {
        samples: perf::since(after),
        uptime_ms: perf::uptime_ms(),
    }
}

/// Timings measured by the frontend (screen transitions, round trips), added to
/// the backend's samples so `perf.jsonl` has the whole picture.
#[tauri::command]
pub fn perf_report(samples: Vec<perf::Sample>) {
    const MAX_BATCH: usize = 500;
    perf::push(
        samples
            .into_iter()
            .take(MAX_BATCH)
            .filter_map(perf::sanitize_ui_sample)
            .collect(),
    );
}

/// Saves the benchmark report to `<bench dir>/perf.json`. Only in `--bench` mode.
#[tauri::command]
pub async fn bench_save(options: State<'_, StartupOptions>, report: String) -> Result<String> {
    const MAX_REPORT: usize = 32 * 1024 * 1024;
    let Some(bench) = &options.bench else {
        return Err(AppError::InvalidInput("not in benchmark mode".into()));
    };
    if report.len() > MAX_REPORT || serde_json::from_str::<serde_json::Value>(&report).is_err() {
        return Err(AppError::InvalidInput("benchmark report".into()));
    }
    let path = PathBuf::from(&bench.dir).join("perf.json");
    let shown = path.display().to_string();
    blocking(move || settings::write_atomic(&path, report.as_bytes())).await?;
    Ok(shown)
}
