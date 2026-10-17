# Builds the benchmark repository used by benchmark.ps1: a large, deterministic history
# (thousands of commits, merges, tags, Japanese text) generated with git fast-import,
# a bare "origin" next to it, and a fixed set of working tree changes.
# Dot-source after _common.ps1: . "$PSScriptRoot\_fixture.ps1"

# Bump when the generated content changes, so existing fixtures are rebuilt.
$FixtureVersion = '1'
$FixtureCommits = 3000
$FixtureFiles = 1200

$script:Utf8 = New-Object System.Text.UTF8Encoding($false)

function Get-FixtureText([int]$File, [int]$Revision, [int]$Lines) {
    $sb = New-Object System.Text.StringBuilder
    for ($i = 0; $i -lt $Lines; $i++) {
        [void]$sb.Append("fn item_${File}_$i() -> u32 { $(($File * 31 + $i * 7 + $Revision) % 1000) } // rev $Revision`n")
    }
    $sb.ToString()
}

function Get-FixturePath([int]$File) {
    "src/module_$($File % 40)/file_$File.rs"
}

# Appends a fast-import `data` block (length in bytes, then the content).
function Add-Data([System.Text.StringBuilder]$Sb, [string]$Text) {
    [void]$Sb.Append("data $($script:Utf8.GetByteCount($Text))`n").Append($Text).Append("`n")
}

function New-FixtureStream([string]$Path) {
    $authors = @('Hana Sato <hana@example.com>', 'Ken Ito <ken@example.com>', 'valneko-twoon2 <valneko-twoon2@example.com>')
    $subjects = @('Fix status parsing', 'ログ表示を高速化', 'Refactor diff renderer', 'ブランチ一覧を改善', 'Add tests for stash', 'Update dependencies')
    $sb = New-Object System.Text.StringBuilder
    $time = 1700000000
    $mark = 0
    $mainMark = 0

    # Initial commit with every file.
    $mark++
    [void]$sb.Append("commit refs/heads/main`nmark :$mark`nauthor $($authors[0]) $time +0900`ncommitter $($authors[0]) $time +0900`n")
    Add-Data $sb "Initial commit`n"
    for ($f = 0; $f -lt $FixtureFiles; $f++) {
        [void]$sb.Append("M 100644 inline $(Get-FixturePath $f)`n")
        Add-Data $sb (Get-FixtureText $f 0 40)
    }
    $mainMark = $mark

    for ($c = 1; $c -lt $FixtureCommits; $c++) {
        $time += 3600
        $author = $authors[$c % $authors.Count]
        $isSide = ($c % 50) -ge 45   # every 50 commits: 4 commits on a side branch, then a merge
        $ref = if ($isSide) { "refs/heads/feature/topic-$([math]::Floor($c / 50))" } else { 'refs/heads/main' }
        $mark++
        [void]$sb.Append("commit $ref`nmark :$mark`nauthor $author $time +0900`ncommitter $author $time +0900`n")
        Add-Data $sb "$($subjects[$c % $subjects.Count]) #$c`n`nGenerated commit $c for the Leveret benchmark.`n"
        if ($isSide -and ($c % 50) -eq 45) { [void]$sb.Append("from :$mainMark`n") }
        elseif (-not $isSide) {
            [void]$sb.Append("from :$mainMark`n")
            if (($c % 50) -eq 0 -and $c -ge 50) { [void]$sb.Append("merge :$($mark - 1)`n") }
        }
        for ($k = 0; $k -lt 3; $k++) {
            $f = ($c * 7 + $k * 131) % $FixtureFiles
            [void]$sb.Append("M 100644 inline $(Get-FixturePath $f)`n")
            Add-Data $sb (Get-FixtureText $f $c 40)
        }
        if (-not $isSide) { $mainMark = $mark }
        if ($c % 500 -eq 0) { [void]$sb.Append("reset refs/tags/v0.$($c / 500).0`nfrom :$mark`n`n") }
    }
    [IO.File]::WriteAllText($Path, $sb.ToString(), $script:Utf8)
}

function Invoke-FixtureGit([string]$Repo, [string[]]$GitArgs) {
    & git -C $Repo @GitArgs
    if ($LASTEXITCODE -ne 0) { throw "git $($GitArgs -join ' ') failed in $Repo" }
}

# Resets the working tree and applies the same changes every run.
function Reset-FixtureChanges([string]$Repo) {
    Invoke-FixtureGit $Repo @('reset', '--hard', '-q', 'main')
    Invoke-FixtureGit $Repo @('clean', '-fdq')
    for ($f = 0; $f -lt 150; $f++) {
        $path = Join-Path $Repo (Get-FixturePath ($f * 7))
        [IO.File]::AppendAllText($path, "// local change $f`nfn changed_$f() {}`n", $script:Utf8)
    }
    for ($f = 0; $f -lt 10; $f++) { Remove-Item (Join-Path $Repo (Get-FixturePath ($f * 11 + 3))) }
    $new = Join-Path $Repo 'new'
    New-Item -ItemType Directory -Force $new | Out-Null
    for ($f = 0; $f -lt 40; $f++) {
        [IO.File]::WriteAllText((Join-Path $new "untracked_$f.txt"), "新しいファイル $f`n", $script:Utf8)
    }
    # One big diff.
    [IO.File]::WriteAllText((Join-Path $Repo (Get-FixturePath 1)), (Get-FixtureText 1 99999 6000), $script:Utf8)
}

# Creates (or reuses) the fixture under $Dir and returns the repository path.
function Initialize-PerfFixture([string]$Dir) {
    $repo = Join-Path $Dir 'repo'
    $stamp = Join-Path $Dir 'version.txt'
    $current = if (Test-Path $stamp) { (Get-Content $stamp -Raw).Trim() } else { '' }
    if ($current -ne $FixtureVersion -or -not (Test-Path (Join-Path $repo '.git'))) {
        Write-Step "Generating the benchmark repository ($FixtureCommits commits, $FixtureFiles files)"
        if (Test-Path $Dir) { Remove-Item -Recurse -Force $Dir }
        New-Item -ItemType Directory -Force $repo | Out-Null
        $stream = Join-Path $Dir 'history.fi'
        New-FixtureStream $stream
        Invoke-FixtureGit $repo @('init', '-q', '-b', 'main')
        $proc = Start-Process git -ArgumentList @('-C', "`"$repo`"", 'fast-import', '--quiet') -RedirectStandardInput $stream -NoNewWindow -Wait -PassThru
        if ($proc.ExitCode -ne 0) { throw 'git fast-import failed' }
        Remove-Item $stream
        $origin = Join-Path $Dir 'origin.git'
        Invoke-FixtureGit $Dir @('clone', '--bare', '-q', $repo, $origin)
        Invoke-FixtureGit $repo @('remote', 'add', 'origin', $origin)
        Invoke-FixtureGit $repo @('fetch', '-q', 'origin')
        Invoke-FixtureGit $repo @('reset', '--hard', '-q', 'main')
        Invoke-FixtureGit $repo @('branch', '-q', '--set-upstream-to=origin/main', 'main')
        Set-Content $stamp $FixtureVersion
    }
    Reset-FixtureChanges $repo
    $repo
}
