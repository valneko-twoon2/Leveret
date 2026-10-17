<#
.SYNOPSIS
  Builds a release and writes artifacts\<version>\Leveret.exe plus a zip of it (Leveret_v<version>.zip).
.PARAMETER SkipTests
  Skip the checks that normally run before building.
#>
param([switch]$SkipTests)
. "$PSScriptRoot\_common.ps1"

$version = Get-AppVersion
$out = Join-Path $Artifacts $version

if (-not $SkipTests) {
    & "$PSScriptRoot\test.ps1"
}

Write-Step "Building Leveret $version"
Invoke-Tauri @('build', '--no-bundle')

Write-Step "Copying outputs to $out"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $out | Out-Null

$exe = Join-Path $out 'Leveret.exe'
Copy-Item (Get-ReleaseExe) $exe
Compress-Archive -Path $exe -DestinationPath (Join-Path $out "Leveret_v$version.zip") -CompressionLevel Optimal

Get-ChildItem $out | Format-Table Name, @{ Name = 'Size(MB)'; Expression = { [math]::Round($_.Length / 1MB, 1) } } -AutoSize
Write-Done "Done: $out"
