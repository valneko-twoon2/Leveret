<#
.SYNOPSIS
  Changes the app version in frontend\package.json, package-lock.json,
  backend\Cargo.toml, Cargo.lock and tauri.conf.json at once.
.PARAMETER Version
  New version (e.g. 0.2.0), or major / minor / patch. Prompts when omitted.
.EXAMPLE
  scripts\version.bat 0.2.0
.EXAMPLE
  scripts\version.bat patch
#>
param([Parameter(Position = 0)][string]$Version)
. "$PSScriptRoot\_common.ps1"

$current = Get-AppVersion
Write-Host "Current version: $current"

if (-not $Version) {
    $Version = Read-Host 'New version (e.g. 0.2.0 / major / minor / patch, empty to cancel)'
    if (-not $Version) { Write-Host 'Cancelled'; return }
}

if ($Version -in @('major', 'minor', 'patch')) {
    if ($current -notmatch '^(\d+)\.(\d+)\.(\d+)') { throw "Cannot parse the current version: $current" }
    $ma = [int]$Matches[1]; $mi = [int]$Matches[2]; $pa = [int]$Matches[3]
    switch ($Version) {
        'major' { $Version = "$($ma + 1).0.0" }
        'minor' { $Version = "$ma.$($mi + 1).0" }
        'patch' { $Version = "$ma.$mi.$($pa + 1)" }
    }
}
if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
    throw "Invalid version: $Version (expected e.g. 1.2.3 or 1.2.3-beta.1)"
}

$utf8 = New-Object System.Text.UTF8Encoding($false)
function Update-File([string]$Path, [string]$Pattern, [string]$Replacement) {
    $text = [IO.File]::ReadAllText($Path, $utf8)
    $regex = New-Object System.Text.RegularExpressions.Regex($Pattern, 'Multiline')
    if (-not $regex.IsMatch($text)) { throw "No version found in $Path" }
    [IO.File]::WriteAllText($Path, $regex.Replace($text, $Replacement, 1), $utf8)
    Write-Host "  updated $($Path.Substring($Root.Length + 1))"
}

Write-Step "$current -> $Version"
Invoke-Native npm @('version', $Version, '--no-git-tag-version', '--allow-same-version') $Frontend | Out-Null
Write-Host '  updated frontend\package.json, frontend\package-lock.json'
Update-File (Join-Path $Backend 'Cargo.toml') '^version = "[^"]*"' "version = `"$Version`""
Update-File (Join-Path $Backend 'tauri.conf.json') '"version": "[^"]*"' "`"version`": `"$Version`""
$lock = Join-Path $Backend 'Cargo.lock'
if (Test-Path $lock) {
    Update-File $lock '(?<=^name = "leveret"\r?\nversion = ")[^"]*' $Version
}

Write-Done "Version changed to $Version"
