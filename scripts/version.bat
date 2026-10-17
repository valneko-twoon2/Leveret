@echo off
rem Runs version.ps1 with the same arguments. See that file for options.
setlocal
set "PS=powershell"
where pwsh >nul 2>nul && set "PS=pwsh"
%PS% -NoProfile -ExecutionPolicy Bypass -File "%~dp0_run.ps1" version %*
