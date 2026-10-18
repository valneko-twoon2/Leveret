<#
.SYNOPSIS
  Saves a screenshot of every screen to artifacts\screenshots. Capturing runs in the background.

  By default it uses the app's built-in capture (leveret.exe --capture), which renders an
  off-screen window and grabs it through WebView2. -Browser uses headless Edge instead
  (no Rust build needed).
.PARAMETER Browser
  Capture with headless Edge.
.PARAMETER Real
  Use the registered real repositories instead of mock data (app capture only).
.PARAMETER Theme
  light, dark, or both (saved to <OutDir>\light and <OutDir>\dark). Follows the OS when omitted.
.PARAMETER Build
  Always rebuild the app first (otherwise it is rebuilt only when sources are newer).
.PARAMETER Scenes
  Scene ids to capture (e.g. 02-changes,04-history). All scenes when omitted.
  See frontend/src/lib/scenes/.
.PARAMETER Size
  Window size for -Browser, e.g. 900x560 (the app's minimum) to check narrow layouts. Default 1280x800.
.PARAMETER OutDir
  Output folder.
.PARAMETER Open
  Open the output folder afterwards.
#>
param(
    [switch]$Browser,
    [switch]$Real,
    [ValidateSet('light', 'dark', 'both')][string]$Theme,
    [switch]$Build,
    [string[]]$Scenes,
    [ValidatePattern('^\d{3,4}x\d{3,4}$')][string]$Size = '1280x800',
    [string]$OutDir,
    [switch]$Open
)
. "$PSScriptRoot\_common.ps1"

if (-not $OutDir) { $OutDir = Join-Path $Artifacts 'screenshots' }
$OutDir = [IO.Path]::GetFullPath($OutDir)
$Scenes = @($Scenes | ForEach-Object { $_ -split ',' } | Where-Object { $_ })

function Get-SceneIds {
    # Scene ids start with their number, so sorting keeps the capture order.
    Get-ChildItem (Join-Path $Frontend 'src\lib\scenes') -Filter '*.ts' | ForEach-Object {
        $source = Get-Content $_.FullName -Raw -Encoding UTF8
        [regex]::Matches($source, "id:\s*'([^']+)'") | ForEach-Object { $_.Groups[1].Value }
    } | Sort-Object
}

function Invoke-AppCapture([string]$Dir, [string]$ThemeName) {
    $appArgs = @('--capture', "`"$Dir`"")
    if (-not $Real) { $appArgs += '--mock' }
    if ($ThemeName) { $appArgs += @('--theme', $ThemeName) }
    if ($Scenes.Count) { $appArgs += @('--scenes', ($Scenes -join ',')) }

    Write-Step "Capturing with the app -> $Dir"
    $proc = Start-Process -FilePath (Get-ReleaseExe) -ArgumentList $appArgs -PassThru
    if (-not $proc.WaitForExit(180000)) {
        $proc.Kill()
        throw 'Capture timed out'
    }
    if ($proc.ExitCode -ne 0) { throw "Some scenes failed to capture (exit code $($proc.ExitCode))" }
}

function Find-Edge {
    $candidates = @(
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
    )
    $edge = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $edge) { throw 'Microsoft Edge was not found' }
    $edge
}

function Invoke-BrowserCapture([string]$BaseUrl, [string]$Dir, [string]$ThemeName) {
    $edge = Find-Edge
    $profileDir = Join-Path ([IO.Path]::GetTempPath()) "leveret-capture-$PID"
    $ids = if ($Scenes.Count) { $Scenes } else { Get-SceneIds }
    Write-Step "Capturing with headless Edge -> $Dir"
    try {
        foreach ($id in $ids) {
            $png = Join-Path $Dir "$id.png"
            $url = "$BaseUrl`?scene=$id"
            if ($ThemeName) { $url += "&theme=$ThemeName" }
            $edgeArgs = @(
                '--headless=new', '--disable-gpu', '--hide-scrollbars', '--no-first-run',
                '--force-device-scale-factor=1', "--window-size=$($Size -replace 'x', ',')", '--virtual-time-budget=15000',
                "--user-data-dir=`"$profileDir`"", "--screenshot=`"$png`"", "`"$url`""
            )
            Start-Process -FilePath $edge -ArgumentList $edgeArgs -Wait -WindowStyle Hidden
            if (Test-Path $png) { Write-Host "  $id.png" } else { Write-Warning "  failed to capture $id" }
        }
    }
    finally {
        Remove-Item -Recurse -Force $profileDir -ErrorAction SilentlyContinue
    }
}

# Themes to capture and their output folders.
$runs = switch ($Theme) {
    'both' { @(@{ Theme = 'light'; Dir = Join-Path $OutDir 'light' }, @{ Theme = 'dark'; Dir = Join-Path $OutDir 'dark' }) }
    { $_ -in 'light', 'dark' } { @(@{ Theme = $Theme; Dir = $OutDir }) }
    default { @(@{ Theme = $null; Dir = $OutDir }) }
}
foreach ($run in $runs) { New-Item -ItemType Directory -Force $run.Dir | Out-Null }

if ($Browser) {
    Initialize-Frontend
    Write-Step 'Building the frontend'
    Invoke-Native npm @('run', 'build') $Frontend

    $port = 4173
    $vite = Join-Path $Frontend 'node_modules\vite\bin\vite.js'
    $server = Start-Process -FilePath node -ArgumentList @("`"$vite`"", 'preview', '--port', $port, '--strictPort') `
        -WorkingDirectory $Frontend -WindowStyle Hidden -PassThru
    try {
        $base = "http://localhost:$port/"
        $up = $false
        for ($i = 0; $i -lt 60 -and -not $up; $i++) {
            try { Invoke-WebRequest -Uri $base -UseBasicParsing -TimeoutSec 1 | Out-Null; $up = $true } catch { Start-Sleep -Milliseconds 500 }
        }
        if (-not $up) { throw 'The preview server did not start' }
        foreach ($run in $runs) { Invoke-BrowserCapture $base $run.Dir $run.Theme }
    }
    finally {
        if (-not $server.HasExited) { Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue }
        Get-CimInstance Win32_Process -Filter "ParentProcessId=$($server.Id)" -ErrorAction SilentlyContinue |
            ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    }
}
else {
    Update-ReleaseExe -Force:$Build
    foreach ($run in $runs) { Invoke-AppCapture $run.Dir $run.Theme }
}

foreach ($run in $runs) {
    Get-ChildItem $run.Dir -Filter *.png | Sort-Object Name | Format-Table @{ Name = 'File'; Expression = { $_.FullName.Substring($OutDir.Length + 1) } }, LastWriteTime -AutoSize
}
Write-Done "Done: $OutDir"
if ($Open) { Invoke-Item $OutDir }
