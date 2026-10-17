<#
.SYNOPSIS
  Checks the development environment and fetches dependencies.
#>
. "$PSScriptRoot\_common.ps1"

Write-Step 'Checking required tools'
Assert-Command git 'Install Git for Windows: https://gitforwindows.org/'
Assert-Command node 'Install Node.js: https://nodejs.org/'
Assert-Command npm 'Install Node.js: https://nodejs.org/'
Assert-Command cargo 'Install Rust: https://rustup.rs/'
git --version
node --version
cargo --version

$webview = Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
if ($webview) { Write-Host "WebView2 $($webview.pv)" } else { Write-Warning 'WebView2 Runtime was not found (it ships with Windows 11)' }

Write-Step 'Enabling git hooks (.githooks)'
Invoke-Native git @('config', 'core.hooksPath', '.githooks') $Root

Write-Step 'Installing npm packages'
Invoke-Native npm @('ci') $Frontend

Write-Step 'Fetching Rust crates'
Invoke-Native cargo @('fetch') $Backend

Write-Done 'Setup complete. Start the app with scripts\dev.bat'
