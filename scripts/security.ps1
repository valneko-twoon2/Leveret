<#
.SYNOPSIS
  Security checks: static rules for risky code and configuration, dependency
  audits (npm audit, cargo audit) and the backend security tests.
.PARAMETER Offline
  Skip the dependency audits (they need network access).
.PARAMETER NoTests
  Skip the backend security tests.
#>
param(
    [switch]$Offline,
    [switch]$NoTests
)
. "$PSScriptRoot\_common.ps1"

$failures = New-Object System.Collections.Generic.List[string]
$warnings = New-Object System.Collections.Generic.List[string]

function Get-SourceFiles([string]$Dir, [string[]]$Extensions) {
    Get-ChildItem -Path $Dir -Recurse -File | Where-Object { $Extensions -contains $_.Extension }
}

function Test-Pattern {
    param([string]$Rule, [IO.FileInfo[]]$Files, [string]$Pattern, [string[]]$AllowedFiles = @())
    foreach ($f in $Files) {
        if ($AllowedFiles | Where-Object { $f.FullName.Replace('\', '/').EndsWith($_) }) { continue }
        $hits = Select-String -Path $f.FullName -Pattern $Pattern -CaseSensitive
        foreach ($h in $hits) {
            $failures.Add("$Rule`n    $($f.FullName.Substring($Root.Length + 1)):$($h.LineNumber): $($h.Line.Trim())")
        }
    }
}

# --- static rules: frontend ---------------------------------------------------
Write-Step 'Static rules: frontend'
$web = @(Get-SourceFiles (Join-Path $Frontend 'src') @('.ts', '.svelte', '.js'))
Test-Pattern 'Raw HTML injection ({@html}, innerHTML, ...) enables XSS' $web '\{@html|\.innerHTML\s*=|\.outerHTML\s*=|insertAdjacentHTML|document\.write'
Test-Pattern 'Dynamic code evaluation' $web '\beval\s*\(|new Function\s*\(|setTimeout\s*\(\s*[''"]|setInterval\s*\(\s*[''"]'
Test-Pattern 'Powerful Tauri plugins must not be used from the webview' $web '@tauri-apps/plugin-(shell|fs|http|process|sql|store)'
Test-Pattern 'Direct asset/file URLs bypass backend validation' $web 'convertFileSrc'
Test-Pattern 'Tokens must stay in the backend (never in web storage)' $web '(localStorage|sessionStorage|indexedDB)[^\n]*(token|secret|password)'

# --- static rules: backend ----------------------------------------------------
Write-Step 'Static rules: backend'
$rs = @(Get-SourceFiles (Join-Path $Backend 'src') @('.rs'))
# difftool.rs starts WinMerge, found through its own registry key (never a path from the webview).
Test-Pattern 'Processes may only be spawned through git::Git (or WinMerge in difftool)' $rs 'Command::new\(' @('src/git/mod.rs', 'src/difftool.rs')
Test-Pattern 'unsafe code is limited to the Win32 / WebView2 integration' $rs '\bunsafe\b' @('src/capture.rs', 'src/lockdown.rs', 'src/crash.rs', 'src/difftool.rs')
Test-Pattern 'Never log or print secrets' $rs '(log::\w+!|println!|eprintln!|dbg!)\([^\n]*(token|secret|password)'
Test-Pattern 'No leftover dbg!' $rs '\bdbg!\('

# --- static rules: configuration ---------------------------------------------
Write-Step 'Static rules: configuration'
# Reads a property that may be absent (StrictMode forbids touching missing ones).
function Get-Prop($Object, [string]$Name) {
    if ($null -ne $Object -and $Object.PSObject.Properties[$Name]) { $Object.$Name } else { $null }
}

$conf = Get-Content (Join-Path $Backend 'tauri.conf.json') -Raw | ConvertFrom-Json
$security = Get-Prop (Get-Prop $conf 'app') 'security'
$csp = Get-Prop $security 'csp'
if (-not $csp) { $failures.Add('tauri.conf.json: a Content-Security-Policy is required') }
elseif ($csp -match "unsafe-eval|script-src[^;]*unsafe-inline|\*") { $failures.Add("tauri.conf.json: CSP is too permissive: $csp") }
if (-not (Get-Prop $security 'freezePrototype')) { $failures.Add('tauri.conf.json: app.security.freezePrototype must be true') }
if (Get-Prop (Get-Prop $conf 'app') 'withGlobalTauri') { $failures.Add('tauri.conf.json: withGlobalTauri exposes the API to any script; keep it off') }
if (Get-Prop (Get-Prop $security 'assetProtocol') 'enable') { $failures.Add('tauri.conf.json: the asset protocol must stay disabled') }
$confText = Get-Content (Join-Path $Backend 'tauri.conf.json') -Raw
if ($confText -match 'dangerous') { $failures.Add('tauri.conf.json: "dangerous*" options are not allowed') }

$cargo = Get-Content (Join-Path $Backend 'Cargo.toml') -Raw
if ($cargo -match 'tauri\s*=\s*\{[^}]*devtools') { $failures.Add('Cargo.toml: the tauri "devtools" feature would enable devtools in release builds') }
if ($cargo -match 'tauri-plugin-(shell|fs|http|process|sql)') { $failures.Add('Cargo.toml: powerful plugins (shell/fs/http/process/sql) are not allowed') }

# Capabilities: only these permissions may be granted to the webview.
$allowed = @('core:event:allow-listen', 'core:event:allow-unlisten', 'dialog:allow-open', 'dialog:allow-message')
foreach ($cap in Get-ChildItem (Join-Path $Backend 'capabilities') -Filter *.json) {
    $json = Get-Content $cap.FullName -Raw | ConvertFrom-Json
    foreach ($perm in $json.permissions) {
        $id = if ($perm -is [string]) { $perm } else { $perm.identifier }
        if ($allowed -notcontains $id) {
            $failures.Add("capabilities/$($cap.Name): permission '$id' is not in the allowlist (scripts/security.ps1)")
        }
    }
}

# --- dependency audits ----------------------------------------------------------
if (-not $Offline) {
    Write-Step 'Dependency audit: npm'
    Initialize-Frontend
    Push-Location $Frontend
    try {
        npm audit --audit-level=high
        if ($LASTEXITCODE -ne 0) { $failures.Add('npm audit reported high or critical vulnerabilities') }
    }
    finally { Pop-Location }

    Write-Step 'Dependency audit: cargo'
    if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
        Push-Location $Backend
        try {
            cargo audit
            if ($LASTEXITCODE -ne 0) { $failures.Add('cargo audit reported vulnerabilities') }
        }
        finally { Pop-Location }
    }
    else {
        $warnings.Add('cargo-audit is not installed: run "cargo install cargo-audit --locked" to audit Rust dependencies')
    }
}

# --- security tests -------------------------------------------------------------
if (-not $NoTests) {
    Write-Step 'Backend security tests'
    Push-Location $Backend
    try {
        cargo test --quiet -- security:: lockdown:: unregistered_repositories
        if ($LASTEXITCODE -ne 0) { $failures.Add('backend security tests failed') }
    }
    finally { Pop-Location }
}

# --- summary --------------------------------------------------------------------
Write-Host ''
foreach ($w in $warnings) { Write-Warning $w }
if ($failures.Count) {
    Write-Host "Security check failed ($($failures.Count)):" -ForegroundColor Red
    foreach ($f in $failures) { Write-Host "  - $f" -ForegroundColor Red }
    throw 'Security check failed'
}
Write-Done 'Security check passed'
