# Shared helpers for the scripts in this folder. Dot-source it: . "$PSScriptRoot\_common.ps1"
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
$Frontend = Join-Path $Root 'frontend'
$Backend = Join-Path $Root 'backend'
$Artifacts = Join-Path $Root 'artifacts'

function Write-Step([string]$Message) {
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Done([string]$Message) {
    Write-Host $Message -ForegroundColor Green
}

# Runs a native command and stops the script when it fails.
function Invoke-Native {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$ArgumentList = @(),
        [string]$WorkingDirectory = $Root
    )
    Push-Location $WorkingDirectory
    try {
        & $FilePath @ArgumentList
        if ($LASTEXITCODE -ne 0) {
            throw "$FilePath $($ArgumentList -join ' ') failed (exit code $LASTEXITCODE)"
        }
    }
    finally {
        Pop-Location
    }
}

function Assert-Command([string]$Name, [string]$Hint) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name was not found. $Hint"
    }
}

# Installs frontend dependencies when node_modules is missing or older than the lock file.
function Initialize-Frontend {
    $modules = Join-Path $Frontend 'node_modules'
    $lock = Join-Path $Frontend 'package-lock.json'
    $stamp = Join-Path $modules '.package-lock.json'
    if (-not (Test-Path $stamp) -or ((Get-Item $lock).LastWriteTime -gt (Get-Item $stamp).LastWriteTime)) {
        Write-Step 'Installing npm packages'
        Invoke-Native npm @('ci') $Frontend
    }
}

# Runs the Tauri CLI (from the frontend's node_modules) inside the backend folder.
function Invoke-Tauri([string[]]$Arguments) {
    Initialize-Frontend
    $cli = Join-Path $Frontend 'node_modules\.bin\tauri.cmd'
    Invoke-Native $cli $Arguments $Backend
}

function Get-AppVersion {
    (Get-Content (Join-Path $Frontend 'package.json') -Raw | ConvertFrom-Json).version
}

# Path of the release executable built by `tauri build`.
function Get-ReleaseExe {
    Join-Path $Backend 'target\release\leveret.exe'
}
