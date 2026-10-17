<#
.SYNOPSIS
  Runs the app in development mode (frontend hot reload, Rust rebuilt on change).
.PARAMETER Mock
  Use mock data instead of real repositories.
#>
param([switch]$Mock)
. "$PSScriptRoot\_common.ps1"

Write-Step 'tauri dev (Ctrl+C to stop)'
$tauriArgs = @('dev')
if ($Mock) { $tauriArgs += @('--', '--', '--mock') }
Invoke-Tauri $tauriArgs
