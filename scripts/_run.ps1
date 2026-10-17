# Entry point used by the .bat wrappers: runs scripts\<name>.ps1 with the remaining
# arguments and keeps the window open when the .bat was started from Explorer.
param(
    [Parameter(Mandatory, Position = 0)][string]$Name,
    [Parameter(ValueFromRemainingArguments)][string[]]$Rest = @()
)

# Arguments arrive as plain strings, so rebuild named/switch parameters using the
# target script's parameter definitions.
$script = Join-Path $PSScriptRoot "$Name.ps1"
$params = (Get-Command $script).Parameters
$named = @{}
$positional = @()
for ($i = 0; $i -lt $Rest.Count; $i++) {
    $arg = $Rest[$i]
    if ($arg -match '^-(\w+)(?::(.*))?$' -and $params.ContainsKey($Matches[1])) {
        $key = $Matches[1]
        if ($null -ne $Matches[2]) { $named[$key] = $Matches[2] }
        elseif ($params[$key].SwitchParameter) { $named[$key] = $true }
        elseif ($i + 1 -lt $Rest.Count) { $named[$key] = $Rest[++$i] }
    }
    else {
        $positional += $arg
    }
}

$code = 0
try {
    & $script @positional @named
}
catch {
    Write-Host ''
    Write-Host "Error: $_" -ForegroundColor Red
    $code = 1
}

try {
    $self = Get-CimInstance Win32_Process -Filter "ProcessId=$PID"
    $cmd = Get-CimInstance Win32_Process -Filter "ProcessId=$($self.ParentProcessId)"
    if ((Get-Process -Id $cmd.ParentProcessId -ErrorAction Stop).ProcessName -eq 'explorer') {
        Read-Host 'Press Enter to close' | Out-Null
    }
}
catch {
    # Parent already gone: not started from Explorer.
}
exit $code
