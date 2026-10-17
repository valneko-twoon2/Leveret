<#
.SYNOPSIS
  Shows the UI only: opens the frontend in a browser without building Rust.
  In a browser the app runs on mock data (frontend/src/lib/mock/) instead of real git.
.PARAMETER Scene
  Capture scene to show first (e.g. 04-history). See frontend/src/lib/capture.ts.
.PARAMETER Theme
  Force the light or dark theme.
.PARAMETER App
  Run as the Tauri app (with mock data) instead of in a browser. Builds Rust.
#>
param(
    [string]$Scene,
    [ValidateSet('light', 'dark')][string]$Theme,
    [switch]$App
)
. "$PSScriptRoot\_common.ps1"

if ($App) {
    Write-Step 'Starting the Tauri app with mock data (Ctrl+C to stop)'
    $appArgs = @('dev', '--', '--', '--mock')
    if ($Theme) { $appArgs += @('--theme', $Theme) }
    Invoke-Tauri $appArgs
    return
}

Initialize-Frontend
$query = @()
if ($Scene) { $query += "scene=$Scene" }
if ($Theme) { $query += "theme=$Theme" }
$url = 'http://localhost:1420/'
if ($query) { $url += '?' + ($query -join '&') }

Write-Step "Opening $url (Ctrl+C to stop)"
# Open the browser once the dev server answers.
Start-Job -ScriptBlock {
    param($u)
    for ($i = 0; $i -lt 60; $i++) {
        try { Invoke-WebRequest -Uri $u -UseBasicParsing -TimeoutSec 1 | Out-Null; Start-Process $u; return } catch { Start-Sleep -Milliseconds 500 }
    }
} -ArgumentList $url | Out-Null

Invoke-Native npm @('run', 'dev') $Frontend
