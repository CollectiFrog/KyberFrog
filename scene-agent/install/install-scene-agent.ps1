# Register (or remove) the kyber-anysource Scene Agent as a logon task.
#
# Run once per scene machine, in the session of the user that will be auto-
# logged-in. The task launches the agent at every logon and Task Scheduler
# relaunches it if the agent process itself ever dies (the agent in turn keeps
# kyclient alive). The agent runs in the interactive desktop session — required
# for the fullscreen GPU client — with no elevation.
#
#   .\install-scene-agent.ps1                 # exe expected next to this script
#   .\install-scene-agent.ps1 -ExePath D:\soft\kyber\kyber-anysource-scene-agent.exe
#   .\install-scene-agent.ps1 -Uninstall
#
# Autologon itself is NOT configured here: it stores a password and is a per-site
# security decision. See README.md.

#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$ExePath,
    [string]$TaskName = "KyberAnysourceSceneAgent",
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

if ($Uninstall) {
    if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
        Write-Host "Removed scheduled task '$TaskName'."
    } else {
        Write-Host "No scheduled task '$TaskName' found."
    }
    return
}

# Resolve the agent executable: explicit -ExePath, else next to this script.
if (-not $ExePath) {
    $ExePath = Join-Path $PSScriptRoot 'kyber-anysource-scene-agent.exe'
}
if (-not (Test-Path $ExePath)) {
    throw "Scene agent exe not found at '$ExePath'. Copy it there or pass -ExePath."
}
$ExePath = (Resolve-Path $ExePath).Path

$user = "$env:USERDOMAIN\$env:USERNAME"

$action  = New-ScheduledTaskAction -Execute $ExePath
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $user
# No execution time limit (tasks otherwise stop after 3 days); relaunch the
# agent process a minute after it exits, indefinitely.
$settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
    -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) `
    -ExecutionTimeLimit ([TimeSpan]::Zero)
# Interactive session, normal privileges — kyclient needs the user's desktop.
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited

Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger `
    -Settings $settings -Principal $principal -Force | Out-Null

Write-Host "Registered '$TaskName' to launch at logon for ${user}:"
Write-Host "  $ExePath"
Write-Host ""
Write-Host "Next: ensure %APPDATA%\kyber-anysource\scene-agent.toml has 'server' set,"
Write-Host "then log off and back on (or run: Start-ScheduledTask -TaskName '$TaskName')."
