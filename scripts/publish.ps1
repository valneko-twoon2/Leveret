<#
.SYNOPSIS
  Builds a release and copies the outputs to artifacts\<version>\.
.PARAMETER NoBundle
  Skip the installers (NSIS / MSI) and only output the executable.
.PARAMETER SkipTests
  Skip the checks that normally run before building.
#>
param(
    [switch]$NoBundle,
    [switch]$SkipTests
)
. "$PSScriptRoot\_common.ps1"

$version = Get-AppVersion
$out = Join-Path $Artifacts $version

if (-not $SkipTests) {
    & "$PSScriptRoot\test.ps1"
}

Write-Step "Building Leveret $version"
$tauriArgs = @('build')
if ($NoBundle) { $tauriArgs += '--no-bundle' }
Invoke-Tauri $tauriArgs

Write-Step "Copying outputs to $out"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $out | Out-Null

Copy-Item (Get-ReleaseExe) (Join-Path $out "Leveret_$version.exe")
if (-not $NoBundle) {
    $bundle = Join-Path $Backend 'target\release\bundle'
    Get-ChildItem -Path (Join-Path $bundle 'nsis'), (Join-Path $bundle 'msi') -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like "*$version*" } |
        Copy-Item -Destination $out
}

Get-ChildItem $out | Format-Table Name, @{ Name = 'Size(MB)'; Expression = { [math]::Round($_.Length / 1MB, 1) } } -AutoSize
Write-Done "Done: $out"
