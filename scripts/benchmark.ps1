<#
.SYNOPSIS
  Benchmark: times every screen of the app and compares the result with the baseline.

  Runs the release exe off-screen with `--bench` (frontend/src/lib/bench.ts) against a large
  generated repository (scripts/_fixture.ps1) using its own config folder, so the user's
  repository list is never touched. Saves artifacts\perf\runs\<time>\perf.json and
  artifacts\perf\latest.json, and prints the median / p95 of every measured step.
.PARAMETER Runs
  How many times every step runs (default 5).
.PARAMETER Repo
  Also benchmark these repositories (e.g. a real project with a GitHub remote, to time fetch).
.PARAMETER Mock
  Use the built-in mock data instead of repositories (measures rendering only).
.PARAMETER SaveBaseline
  Store this result as artifacts\perf\baseline.json, the reference for later runs.
.PARAMETER Strict
  Fail when a step exceeds its budget or is noticeably slower than the baseline.
.PARAMETER Build
  Always rebuild the app first (otherwise it is rebuilt only when sources are newer).
#>
param(
    [int]$Runs = 5,
    [string[]]$Repo = @(),
    [switch]$Mock,
    [switch]$SaveBaseline,
    [switch]$Strict,
    [switch]$Build
)
. "$PSScriptRoot\_common.ps1"
. "$PSScriptRoot\_fixture.ps1"

# Slower than the baseline by more than this factor and this many milliseconds counts as a regression.
$RegressionFactor = 1.25
$RegressionMinMs = 15

$perfDir = Join-Path $Artifacts 'perf'
$runDir = Join-Path $perfDir ('runs\' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
$configDir = Join-Path $perfDir 'config'
$baselineFile = Join-Path $perfDir 'baseline.json'
New-Item -ItemType Directory -Force $runDir, $configDir | Out-Null

Update-ReleaseExe -Force:$Build

# Repository list for the benchmark's own config folder.
$repos = @()
if (-not $Mock) {
    $repos += Initialize-PerfFixture (Join-Path $perfDir 'fixture')
    foreach ($r in ($Repo | ForEach-Object { $_ -split ',' } | Where-Object { $_ })) {
        $top = (& git -C $r rev-parse --show-toplevel).Trim() -replace '/', '\'
        if ($LASTEXITCODE -ne 0) { throw "$r is not a git repository" }
        $repos += $top
    }
}
$list = @{
    repos    = @($repos | ForEach-Object { @{ path = $_; name = (Split-Path $_ -Leaf) } })
    selected = $repos | Select-Object -First 1
}
[IO.File]::WriteAllText((Join-Path $configDir 'repos.json'), ($list | ConvertTo-Json -Depth 4), $script:Utf8)
Remove-Item (Join-Path $configDir 'settings.json') -ErrorAction SilentlyContinue

Write-Step "Running the benchmark ($Runs runs, $(if ($Mock) { 'mock data' } else { "$($repos.Count) repositories" }))"
$appArgs = @('--bench', "`"$runDir`"", '--runs', $Runs)
if ($Mock) { $appArgs += '--mock' }
$previous = $env:LEVERET_CONFIG_DIR
$env:LEVERET_CONFIG_DIR = $configDir
try {
    $proc = Start-Process -FilePath (Get-ReleaseExe) -ArgumentList $appArgs -PassThru
    if (-not $proc.WaitForExit(900000)) {
        $proc.Kill()
        throw 'The benchmark timed out'
    }
    if ($proc.ExitCode -ne 0) { throw "The benchmark failed (exit code $($proc.ExitCode))" }
}
finally {
    $env:LEVERET_CONFIG_DIR = $previous
}

$reportFile = Join-Path $runDir 'perf.json'
$report = Get-Content $reportFile -Raw -Encoding UTF8 | ConvertFrom-Json
Copy-Item $reportFile (Join-Path $perfDir 'latest.json') -Force

# Baseline medians by kind and name.
$baseline = @{}
if ((Test-Path $baselineFile) -and -not $SaveBaseline) {
    foreach ($s in (Get-Content $baselineFile -Raw -Encoding UTF8 | ConvertFrom-Json).summary) {
        $baseline["$($s.kind) $($s.name)"] = [double]$s.median
    }
}

$problems = New-Object System.Collections.Generic.List[string]
$kindOrder = @{ startup = 0; view = 1; action = 2; ipc = 3; git = 4; http = 5; tool = 6; longtask = 7 }
$rows = foreach ($s in ($report.summary | Sort-Object @{ Expression = { $kindOrder[$_.kind] } }, @{ Expression = { $_.p95 }; Descending = $true })) {
    $key = "$($s.kind) $($s.name)"
    $change = ''
    if ($baseline.ContainsKey($key) -and $baseline[$key] -gt 0) {
        $base = $baseline[$key]
        $change = '{0:+0;-0}%' -f (($s.median - $base) / $base * 100)
        if ($s.median -gt $base * $RegressionFactor -and $s.median - $base -gt $RegressionMinMs) {
            $problems.Add("slower than the baseline: $key median $($s.median) ms (was $base ms)")
        }
    }
    if ($s.overBudget) { $problems.Add("over budget: $key p95 $($s.p95) ms (budget $($s.budget) ms)") }
    [pscustomobject]@{
        Kind     = $s.kind
        Name     = $s.name
        Count    = $s.count
        Median   = $s.median
        P95      = $s.p95
        Max      = $s.max
        Budget   = if ($null -ne $s.budget) { $s.budget } else { '' }
        Status   = if ($s.overBudget) { 'OVER' } else { '' }
        Baseline = $change
    }
}
$rows | Format-Table -AutoSize | Out-String -Width 200 | Write-Host

if ($SaveBaseline) {
    Copy-Item $reportFile $baselineFile -Force
    Write-Done "Saved as the baseline: $baselineFile"
}
foreach ($p in $problems) { Write-Warning $p }
if ($Strict -and $problems.Count) { throw "$($problems.Count) performance problem(s)" }
Write-Done "Report: $reportFile"
