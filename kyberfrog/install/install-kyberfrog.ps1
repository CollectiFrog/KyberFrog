# Register (or remove) KyberFrog as a logon task.
#
# Run once per machine, in the session of the user that will be auto-logged-in.
# The task launches KyberFrog at every logon and Task Scheduler relaunches it if
# the process itself ever dies (KyberFrog in turn keeps its kycontroller and
# kyclient children alive). It runs in the interactive desktop session —
# required for the fullscreen GPU viewers — with no elevation.
#
#   .\install-kyberfrog.ps1                 # exe expected next to this script
#   .\install-kyberfrog.ps1 -ExePath "C:\Program Files\KyberFrog\kyberfrog.exe"
#   .\install-kyberfrog.ps1 -Uninstall
#
# Autologon itself is NOT configured here: it stores a password and is a per-site
# security decision. See README.md.

#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$ExePath,
    [string]$TaskName = "KyberFrog",
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

# Resolve the executable: explicit -ExePath, else next to this script.
if (-not $ExePath) {
    $ExePath = Join-Path $PSScriptRoot 'kyberfrog.exe'
}
if (-not (Test-Path $ExePath)) {
    throw "KyberFrog exe not found at '$ExePath'. Copy it there or pass -ExePath."
}
$ExePath = (Resolve-Path $ExePath).Path

$user = "$env:USERDOMAIN\$env:USERNAME"

$action  = New-ScheduledTaskAction -Execute $ExePath
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $user
# No execution time limit (tasks otherwise stop after 3 days); relaunch the
# process a minute after it exits, indefinitely.
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
Write-Host "Next: open http://localhost:7700/ to add transmitters and/or viewers,"
Write-Host "then log off and back on (or run: Start-ScheduledTask -TaskName '$TaskName')."
