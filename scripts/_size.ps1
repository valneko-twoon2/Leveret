# File size limits (AGENTS.md, "File size"): large files are split by responsibility
# instead of growing further. Dot-source after _common.ps1 and call Test-FileSizes.

$SizeLimits = @(
    @{ Kind = 'source'; Extensions = @('.rs', '.ts', '.svelte', '.js', '.css', '.ps1'); Lines = 500; Bytes = 32KB },
    @{ Kind = 'document'; Extensions = @('.md'); Lines = 400; Bytes = 32KB }
)

# Returns one message per tracked file over its limit.
function Get-OversizedFiles {
    $files = & git -C $Root ls-files --cached --others --exclude-standard
    if ($LASTEXITCODE -ne 0) { throw 'git ls-files failed' }
    foreach ($rel in $files) {
        $ext = [IO.Path]::GetExtension($rel).ToLowerInvariant()
        $limit = $SizeLimits | Where-Object { $_.Extensions -contains $ext } | Select-Object -First 1
        $path = Join-Path $Root $rel
        if (-not $limit -or -not (Test-Path $path -PathType Leaf)) { continue }
        $bytes = (Get-Item $path).Length
        $lines = [IO.File]::ReadAllLines($path).Count
        if ($lines -gt $limit.Lines -or $bytes -gt $limit.Bytes) {
            "$rel`: $lines lines, $([math]::Round($bytes / 1KB)) KB (limit for a $($limit.Kind): $($limit.Lines) lines, $($limit.Bytes / 1KB) KB)"
        }
    }
}

function Test-FileSizes {
    Write-Step 'Checking file sizes'
    $over = @(Get-OversizedFiles)
    if ($over.Count) {
        $over | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
        throw "$($over.Count) file(s) are too large: split them by responsibility (see AGENTS.md, File size)"
    }
}
