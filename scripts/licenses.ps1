<#
.SYNOPSIS
  Collects the licenses of the open source packages shipped in Leveret.
.DESCRIPTION
  Rust: the normal (not build or dev) dependencies of the backend for Windows,
  from `cargo metadata`. npm: the frontend's runtime dependencies plus Svelte,
  whose runtime is bundled. License texts are read from each package
  (LICENSE*, LICENCE*, COPYING*, NOTICE*) and stored once per distinct text.

  Writes frontend/src/lib/licenses/third-party.json, shown in Settings > Licenses.
  Run it after changing dependencies; -Check fails when the file is out of date.
.PARAMETER Check
  Only compare with the existing file (used by test.bat).
#>
param([switch]$Check)
. "$PSScriptRoot\_common.ps1"

$Out = Join-Path $Frontend 'src\lib\licenses\third-party.json'

# Windows PowerShell 5 parses and writes JSON differently (encodings, escaping),
# so the list is only made and checked with PowerShell 7 (pwsh).
if ($PSVersionTable.PSVersion.Major -lt 7) {
    if ($Check) {
        Write-Warning 'License list not checked: install PowerShell 7 (pwsh)'
        return
    }
    throw 'licenses.ps1 needs PowerShell 7 (pwsh): https://aka.ms/powershell'
}

# The license files of a package, one text per file.
function Get-LicenseTexts([string]$Dir) {
    if (-not (Test-Path $Dir)) { return @() }
    $files = Get-ChildItem -LiteralPath $Dir -File |
        Where-Object { $_.Name -match '^(LICEN[CS]E|COPYING|NOTICE)([-._].*)?$' } |
        Sort-Object Name
    return @(foreach ($f in $files) { ([IO.File]::ReadAllText($f.FullName) -replace "`r`n", "`n").Trim() })
}

# --- Rust ---------------------------------------------------------------------
Write-Step 'Reading Rust dependencies (cargo metadata)'
$metaJson = & cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-msvc --locked --offline --manifest-path (Join-Path $Backend 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$meta = $metaJson | ConvertFrom-Json
$byId = @{}
foreach ($p in $meta.packages) { $byId[$p.id] = $p }
$nodes = @{}
foreach ($n in $meta.resolve.nodes) { $nodes[$n.id] = $n }

# Walk normal dependencies from the root package.
$seen = @{}
$queue = [Collections.Generic.Queue[string]]::new()
$queue.Enqueue($meta.resolve.root)
while ($queue.Count) {
    $id = $queue.Dequeue()
    foreach ($d in $nodes[$id].deps) {
        $normal = @($d.dep_kinds | Where-Object { $null -eq $_.kind }).Count -gt 0
        if ($normal -and -not $seen.ContainsKey($d.pkg)) {
            $seen[$d.pkg] = $true
            $queue.Enqueue($d.pkg)
        }
    }
}

$packages = [Collections.Generic.List[object]]::new()
foreach ($id in $seen.Keys) {
    $p = $byId[$id]
    $packages.Add([pscustomobject]@{
        ecosystem = 'rust'
        name = $p.name
        version = $p.version
        license = if ($p.license) { $p.license } elseif ($p.license_file) { 'see license file' } else { 'unknown' }
        repository = if ($p.repository) { $p.repository } else { '' }
        dir = Split-Path -Parent $p.manifest_path
    })
}

# --- npm ----------------------------------------------------------------------
Write-Step 'Reading npm dependencies (package-lock.json)'
# The lock file's root package has an empty key, which ConvertFrom-Json rejects.
$lockText = (Get-Content (Join-Path $Frontend 'package-lock.json') -Raw) -replace '""\s*:\s*\{', '"<root>": {'
$lock = $lockText | ConvertFrom-Json
$lockPackages = @{}
foreach ($prop in $lock.packages.PSObject.Properties) { $lockPackages[$prop.Name] = $prop.Value }
$manifest = Get-Content (Join-Path $Frontend 'package.json') -Raw | ConvertFrom-Json

# node's lookup: <from>/node_modules/<name>, then each parent's node_modules.
function Resolve-LockPath([string]$From, [string]$Name) {
    $base = $From
    while ($true) {
        $candidate = if ($base) { "$base/node_modules/$Name" } else { "node_modules/$Name" }
        if ($lockPackages.ContainsKey($candidate)) { return $candidate }
        if (-not $base) { return $null }
        $i = $base.LastIndexOf('/node_modules/')
        $base = if ($i -ge 0) { $base.Substring(0, $i) } else { '' }
    }
}

$roots = @($manifest.dependencies.PSObject.Properties.Name) + @('svelte')
$npmSeen = @{}
$npmQueue = [Collections.Generic.Queue[string]]::new()
foreach ($name in $roots) {
    $path = Resolve-LockPath '' $name
    if ($path -and -not $npmSeen.ContainsKey($path)) { $npmSeen[$path] = $true; $npmQueue.Enqueue($path) }
}
while ($npmQueue.Count) {
    $path = $npmQueue.Dequeue()
    $entry = $lockPackages[$path]
    if (-not $entry.PSObject.Properties['dependencies']) { continue }
    foreach ($dep in $entry.dependencies.PSObject.Properties.Name) {
        $child = Resolve-LockPath $path $dep
        if ($child -and -not $npmSeen.ContainsKey($child)) { $npmSeen[$child] = $true; $npmQueue.Enqueue($child) }
    }
}

foreach ($path in $npmSeen.Keys) {
    $entry = $lockPackages[$path]
    $dir = Join-Path $Frontend ($path -replace '/', '\')
    $name = $path.Substring($path.LastIndexOf('node_modules/') + 'node_modules/'.Length)
    $repo = ''
    $pkgJson = Join-Path $dir 'package.json'
    if (Test-Path $pkgJson) {
        $pkg = Get-Content $pkgJson -Raw | ConvertFrom-Json
        if ($pkg.PSObject.Properties['repository']) {
            $repo = if ($pkg.repository -is [string]) { $pkg.repository } else { [string]$pkg.repository.url }
            $repo = $repo -replace '^git\+', '' -replace '\.git$', '' -replace '^git://', 'https://'
        }
    }
    $packages.Add([pscustomobject]@{
        ecosystem = 'npm'
        name = $name
        version = $entry.version
        license = if ($entry.PSObject.Properties['license']) { $entry.license } else { 'unknown' }
        repository = $repo
        dir = $dir
    })
}

# --- texts and output ---------------------------------------------------------
Write-Step "Collecting license texts ($($packages.Count) packages)"
$texts = [Collections.Generic.List[string]]::new()
$textIndex = @{}
$entries = foreach ($p in ($packages | Sort-Object ecosystem, name, version)) {
    # Texts that only differ in whitespace (common for Apache-2.0) are stored once.
    $indexes = @(foreach ($text in (Get-LicenseTexts $p.dir)) {
        if (-not $text) { continue }
        $key = ($text -replace '\s+', ' ')
        if (-not $textIndex.ContainsKey($key)) {
            $textIndex[$key] = $texts.Count
            $texts.Add($text)
        }
        $textIndex[$key]
    })
    [ordered]@{
        ecosystem = $p.ecosystem
        name = $p.name
        version = $p.version
        license = $p.license
        repository = $p.repository
        texts = $indexes
    }
}

$self = [IO.File]::ReadAllText((Join-Path $Root 'LICENSE')) -replace "`r`n", "`n"
$data = [ordered]@{ self = $self.Trim(); packages = @($entries); texts = @($texts) }
$json = ($data | ConvertTo-Json -Depth 5 -Compress) + "`n"

if ($Check) {
    # Compared as data: Windows PowerShell and PowerShell 7 escape JSON differently.
    $current = if (Test-Path $Out) { [IO.File]::ReadAllText($Out) | ConvertFrom-Json | ConvertTo-Json -Depth 5 -Compress } else { '' }
    $expected = $json | ConvertFrom-Json | ConvertTo-Json -Depth 5 -Compress
    if ($current -ne $expected) {
        throw 'frontend/src/lib/licenses/third-party.json is out of date: run scripts\licenses.bat'
    }
    Write-Done 'License list is up to date'
    return
}
New-Item -ItemType Directory -Force (Split-Path -Parent $Out) | Out-Null
[IO.File]::WriteAllText($Out, $json, [Text.UTF8Encoding]::new($false))
$kb = [math]::Round((Get-Item $Out).Length / 1KB)
Write-Done "Wrote $Out ($($packages.Count) packages, $($texts.Count) distinct texts, $kb KB)"
