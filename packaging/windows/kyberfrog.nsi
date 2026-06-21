; KyberFrog installer — bundles kyberfrog.exe + the Kyber fork binaries into one
; self-contained, double-click setup. Installs to Program Files, adds the folder
; to the machine PATH (so kyclient/kycontroller resolve), creates Start-Menu
; shortcuts, optionally registers the logon autostart task, and ships an
; uninstaller. Adapted from the upstream kyber-installer NSIS script, minus the
; service / encoder / TLS / auth wizard pages — KyberFrog manages its own config
; (%APPDATA%\kyberfrog\kyberfrog.toml) and uses the transparent default login.

!define PRODUCT_NAME "KyberFrog"
!ifndef PRODUCT_VERSION
    !define PRODUCT_VERSION "0.0.0-dev"
!endif
!define PRODUCT_PUBLISHER "Tristan Perrault"
!define PRODUCT_WEB_SITE "https://gitlab.com/kyber-frog/kyberfrog"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
!define DASHBOARD_URL "http://localhost:7700/"

; Required defines (passed via makensis -D from build-installer.sh):
;   STAGING_DIR  — folder holding everything to install (exe + fork + plugins\)
;   OUTPUT_DIR   — where to drop the built setup .exe
;   OUTPUT_NAME  — file name of the built setup .exe

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "${OUTPUT_DIR}\${OUTPUT_NAME}"
InstallDir "$PROGRAMFILES64\KyberFrog"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
SetOverwrite ifdiff

!define MUI_ICON "${STAGING_DIR}\kyberfrog.ico"
!define MUI_UNICON "${STAGING_DIR}\kyberfrog.ico"
!define MUI_ABORTWARNING

!include "MUI2.nsh"
!include "nsDialogs.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "WinMessages.nsh"

; Options-page variables
Var Dialog
Var AutostartCheckbox
Var Autostart            ; ${BST_CHECKED} / unchecked
Var RunNowCheckbox
Var RunNow
; Uninstall-page variables
Var KeepConfigCheckbox
Var KeepConfig

;--------------------------------
; Pages
;--------------------------------
!define MUI_WELCOMEPAGE_TEXT "This will install KyberFrog and the bundled Kyber streaming binaries on this machine.$\r$\n$\r$\nOne app, both roles — the machine emits, receives, or both, set later from the web UI (${DASHBOARD_URL}).$\r$\n$\r$\nNo separate Kyber install or manual PATH step is needed: everything is included.$\r$\n$\r$\nClick Next to continue."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGING_DIR}\license.txt"
!insertmacro MUI_PAGE_DIRECTORY
Page custom OptionsPage OptionsPageLeave
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION FinishRun
!define MUI_FINISHPAGE_RUN_TEXT "Launch KyberFrog now"
!define MUI_FINISHPAGE_SHOWREADME "${DASHBOARD_URL}"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Open the dashboard (${DASHBOARD_URL})"
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED
!define MUI_FINISHPAGE_LINK "KyberFrog on GitLab"
!define MUI_FINISHPAGE_LINK_LOCATION "${PRODUCT_WEB_SITE}"
!insertmacro MUI_PAGE_FINISH

UninstPage custom un.OptionsPage un.OptionsPageLeave
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

;--------------------------------
; Init
;--------------------------------
Function .onInit
    SetRegView 64
    StrCpy $Autostart ${BST_UNCHECKED}
    StrCpy $RunNow ${BST_CHECKED}

    ; Silent-install flags: /AUTOSTART=1 registers the logon task.
    ${GetParameters} $R0
    ${GetOptions} $R0 "/AUTOSTART=" $R1
    ${IfNot} ${Errors}
        ${If} $R1 == "1"
            StrCpy $Autostart ${BST_CHECKED}
        ${EndIf}
    ${EndIf}

    ; Uninstall a previous version first (keeps the user's %APPDATA% config).
    ReadRegStr $0 HKLM "${PRODUCT_UNINST_KEY}" "UninstallString"
    ${If} $0 != ""
        ReadRegStr $2 HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion"
        ${IfNot} ${Silent}
            MessageBox MB_YESNO "${PRODUCT_NAME} $2 is already installed. Replace it with ${PRODUCT_VERSION}?" IDNO skip_prev
        ${EndIf}
        ReadRegStr $1 HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation"
        ExecWait '$0 /S /KEEPCONFIG=1 _?=$1'
        skip_prev:
    ${EndIf}
FunctionEnd

;--------------------------------
; Options page (autostart + run now)
;--------------------------------
Function OptionsPage
    ${If} ${Silent}
        Abort
    ${EndIf}

    !insertmacro MUI_HEADER_TEXT "Options" "Startup behaviour."
    nsDialogs::Create 1018
    Pop $Dialog

    ${NSD_CreateGroupBox} 0 0 100% 60u "Autostart (for hands-off display PCs)"
    Pop $0
    ${NSD_CreateLabel} 10u 14u 290u 26u "Register a logon task so KyberFrog launches automatically at every logon and is relaunched if it ever exits. Recommended on a dedicated display machine; leave off on a regie/laptop you start by hand."
    Pop $0
    ${NSD_CreateCheckbox} 10u 42u 290u 12u "Launch KyberFrog at logon"
    Pop $AutostartCheckbox
    ${If} $Autostart == ${BST_CHECKED}
        ${NSD_Check} $AutostartCheckbox
    ${EndIf}

    ${NSD_CreateCheckbox} 0 70u 100% 12u "Launch KyberFrog when the installer finishes"
    Pop $RunNowCheckbox
    ${If} $RunNow == ${BST_CHECKED}
        ${NSD_Check} $RunNowCheckbox
    ${EndIf}

    nsDialogs::Show
FunctionEnd

Function OptionsPageLeave
    ${NSD_GetState} $AutostartCheckbox $Autostart
    ${NSD_GetState} $RunNowCheckbox $RunNow
FunctionEnd

Function FinishRun
    Exec '"$INSTDIR\kyberfrog.exe"'
FunctionEnd

;--------------------------------
; Install
;--------------------------------
Section "KyberFrog" SEC_MAIN
    SectionIn RO
    SetOutPath "$INSTDIR"

    ; Everything staged by build-installer.sh: kyberfrog.exe, the fork binaries,
    ; all DLLs, the icon, the autostart script, license.
    File "${STAGING_DIR}\*.*"

    ; libVLC plugins (kyclient's video path needs the plugins\ folder next to it).
    SetOutPath "$INSTDIR\plugins"
    File /r "${STAGING_DIR}\plugins\*.*"
    SetOutPath "$INSTDIR"

    ; Built web UI (React app) — kyberfrog.exe serves the dashboard from ui\dist.
    SetOutPath "$INSTDIR\ui\dist"
    File /r "${STAGING_DIR}\ui\dist\*.*"
    SetOutPath "$INSTDIR"

    ; kycontroller writes its own log4rs file to <exe_dir>\log\ (exe-relative on
    ; Windows, NOT cwd). In read-only Program Files that write fails — exit 101 —
    ; without admin, which also breaks the non-elevated autostart task. Pre-create
    ; the folder and grant the Users group Modify on just log\ (nothing there is
    ; ever executed or loaded, so no DLL-hijack surface). The well-known SID
    ; S-1-5-32-545 = Builtin\Users, locale-independent. Proper long-term fix is
    ; fork-side (log to %LOCALAPPDATA%, like kyclient) — see IMPROVEMENTS.md #9.
    CreateDirectory "$INSTDIR\log"
    nsExec::ExecToLog 'icacls "$INSTDIR\log" /grant *S-1-5-32-545:(OI)(CI)M /T'

    ; Start-Menu shortcuts.
    CreateDirectory "$SMPROGRAMS\KyberFrog"
    CreateShortcut "$SMPROGRAMS\KyberFrog\KyberFrog.lnk" "$INSTDIR\kyberfrog.exe" "" "$INSTDIR\kyberfrog.ico"
    CreateShortcut "$SMPROGRAMS\KyberFrog\KyberFrog Dashboard.lnk" "${DASHBOARD_URL}" "" "$INSTDIR\kyberfrog.ico"
    CreateShortcut "$SMPROGRAMS\KyberFrog\Uninstall KyberFrog.lnk" "$INSTDIR\uninstall.exe"

    ; Register the logon autostart task if requested (runs in the installing
    ; user's interactive session — required for the fullscreen GPU viewers).
    ${If} $Autostart == ${BST_CHECKED}
        DetailPrint "Registering the logon autostart task..."
        nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\install-kyberfrog.ps1" -ExePath "$INSTDIR\kyberfrog.exe"'
        Pop $0
        ${If} $0 != 0
            DetailPrint "Warning: autostart task registration failed (exit $0). Run install-kyberfrog.ps1 manually as the display user."
        ${EndIf}
    ${EndIf}
SectionEnd

Section "-Finalize"
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Add $INSTDIR to the machine PATH via .NET (handles PATH > NSIS_MAX_STRLEN
    ; 1024 safely; the classic EnvVarUpdate.nsh truncates and destroys PATH).
    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -Command " \
        $$dir = \"$INSTDIR\"; \
        $$path = [Environment]::GetEnvironmentVariable(\"Path\", \"Machine\"); \
        if ($$path -split \";\" -inotcontains $$dir) { \
            [Environment]::SetEnvironmentVariable(\"Path\", \"$$path;$$dir\", \"Machine\") \
        }"'
    Pop $0
    ${If} $0 != 0
        DetailPrint "Warning: failed to add $INSTDIR to PATH (exit $0). Add it manually if kyclient/kycontroller aren't found."
    ${EndIf}
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\kyberfrog.ico"
    WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoModify" 1
    WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoRepair" 1
SectionEnd

;--------------------------------
; Uninstall
;--------------------------------
Function un.onInit
    SetRegView 64
    StrCpy $KeepConfig ${BST_CHECKED}    ; keep %APPDATA% config by default

    ${GetParameters} $R0
    ${GetOptions} $R0 "/KEEPCONFIG=" $R1
    ${IfNot} ${Errors}
        ${If} $R1 == "0"
            StrCpy $KeepConfig ${BST_UNCHECKED}
        ${EndIf}
    ${EndIf}
FunctionEnd

Function un.OptionsPage
    ${If} ${Silent}
        Abort
    ${EndIf}
    !insertmacro MUI_HEADER_TEXT "Uninstall Options" "Choose what to keep."
    nsDialogs::Create 1018
    Pop $Dialog
    ${NSD_CreateCheckbox} 0 10u 100% 24u "Keep my data (%APPDATA%\kyberfrog: kyberfrog.toml, logs, per-instance dirs; and %LOCALAPPDATA%\kyber: kyclient known_hosts + logs)"
    Pop $KeepConfigCheckbox
    ${If} $KeepConfig == ${BST_CHECKED}
        ${NSD_Check} $KeepConfigCheckbox
    ${EndIf}
    nsDialogs::Show
FunctionEnd

Function un.OptionsPageLeave
    ${NSD_GetState} $KeepConfigCheckbox $KeepConfig
FunctionEnd

Section "Uninstall"
    SetRegView 64

    ; Remove the autostart task, then stop KyberFrog and any children it spawned.
    DetailPrint "Removing the autostart task (if any)..."
    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\install-kyberfrog.ps1" -Uninstall'
    DetailPrint "Stopping KyberFrog and children..."
    nsExec::ExecToLog 'taskkill /F /IM kyberfrog.exe'
    nsExec::ExecToLog 'taskkill /F /IM kycontroller.exe'
    nsExec::ExecToLog 'taskkill /F /IM kyavserver.exe'
    nsExec::ExecToLog 'taskkill /F /IM kyclient.exe'

    ; Remove $INSTDIR from the machine PATH.
    nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -Command " \
        $$dir = \"$INSTDIR\"; \
        $$path = [Environment]::GetEnvironmentVariable(\"Path\", \"Machine\"); \
        $$new = ($$path -split \";\" | Where-Object { $$_ -ine $$dir }) -join \";\"; \
        [Environment]::SetEnvironmentVariable(\"Path\", $$new, \"Machine\")"'
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

    ; Release the lock on $INSTDIR before deleting it.
    SetOutPath "$TEMP"

    Delete "$INSTDIR\*.exe"
    Delete "$INSTDIR\*.dll"
    Delete "$INSTDIR\*.toml"
    Delete "$INSTDIR\*.pem"
    Delete "$INSTDIR\*.bat"
    Delete "$INSTDIR\*.ps1"
    Delete "$INSTDIR\*.ico"
    Delete "$INSTDIR\license.txt"
    Delete "$INSTDIR\INSTALL.md"
    RMDir /r "$INSTDIR\plugins"
    RMDir /r "$INSTDIR\ui"
    RMDir /r "$INSTDIR\log"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"
    ${If} ${FileExists} "$INSTDIR"
        RMDir /REBOOTOK "$INSTDIR"
    ${EndIf}

    ; Optionally wipe the user's data directories: KyberFrog's own config/logs
    ; (%APPDATA%\kyberfrog) and Kyber's per-user state written by kyclient
    ; (%LOCALAPPDATA%\kyber: TOFU known_hosts + kyclient logs).
    ${If} $KeepConfig != ${BST_CHECKED}
        DetailPrint "Removing %APPDATA%\kyberfrog..."
        RMDir /r "$APPDATA\kyberfrog"
        DetailPrint "Removing %LOCALAPPDATA%\kyber..."
        RMDir /r "$LOCALAPPDATA\kyber"
    ${EndIf}

    Delete "$SMPROGRAMS\KyberFrog\*.lnk"
    RMDir "$SMPROGRAMS\KyberFrog"
    DeleteRegKey HKLM "${PRODUCT_UNINST_KEY}"
SectionEnd
