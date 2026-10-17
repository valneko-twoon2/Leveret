<#
.SYNOPSIS
  Runs tests and static checks (svelte-check, frontend build, cargo test, cargo clippy).
.PARAMETER Fast
  Skip clippy.
#>
param([switch]$Fast)
. "$PSScriptRoot\_common.ps1"

Initialize-Frontend

# Static security rules (offline; the full check with audits is scripts\security.bat).
& "$PSScriptRoot\security.ps1" -Offline -NoTests

Write-Step 'Type-checking the frontend (svelte-check)'
Invoke-Native npm @('run', 'check') $Frontend

Write-Step 'Building the frontend'
Invoke-Native npm @('run', 'build') $Frontend

Write-Step 'Testing the backend (cargo test)'
Invoke-Native cargo @('test') $Backend

if (-not $Fast) {
    Write-Step 'Linting the backend (cargo clippy)'
    Invoke-Native cargo @('clippy', '--all-targets', '--', '-D', 'warnings') $Backend
}

Write-Done 'All checks passed'
