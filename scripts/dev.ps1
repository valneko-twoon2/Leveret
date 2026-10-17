<#
.SYNOPSIS
  Runs the app in development mode (frontend hot reload, Rust rebuilt on change).
  Timings are written to %LOCALAPPDATA%\com.leveret.desktop\logs\perf.jsonl.
.PARAMETER Mock
  Use mock data instead of real repositories.
#>
param([switch]$Mock)
. "$PSScriptRoot\_common.ps1"

# Record every timing to %LOCALAPPDATA%\com.leveret.desktop\logs\perf.jsonl (see backend/src/perf.rs).
$env:LEVERET_PERF = '1'
Write-Step 'tauri dev (Ctrl+C to stop)'
$tauriArgs = @('dev')
if ($Mock) { $tauriArgs += @('--', '--', '--mock') }
Invoke-Tauri $tauriArgs
