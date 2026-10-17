<#
.SYNOPSIS
  Deletes build outputs.
.PARAMETER All
  Also delete node_modules and the Rust build cache (backend\target).
#>
param([switch]$All)
. "$PSScriptRoot\_common.ps1"

$targets = @((Join-Path $Frontend 'dist'), $Artifacts)
if ($All) {
    $targets += @((Join-Path $Frontend 'node_modules'), (Join-Path $Backend 'target'))
}
foreach ($t in $targets) {
    if (Test-Path $t) {
        Write-Host "Removing $t"
        Remove-Item -Recurse -Force $t
    }
}
Write-Done 'Done'
