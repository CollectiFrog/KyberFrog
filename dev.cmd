@echo off
rem dev.cmd - PowerShell / cmd entry point: runs dev.sh through Git Bash.
rem   .\dev setup      .\dev test      .\dev installer      .\dev help
rem
rem A .cmd rather than a .ps1: PowerShell's default execution policy on Windows
rem client blocks .ps1 scripts, never .cmd. Git Bash rather than a bare "bash":
rem that one may be WSL's, which sees another filesystem and no Docker Desktop.
setlocal

set "GIT_EXE="
for /f "delims=" %%G in ('where git 2^>nul') do if not defined GIT_EXE set "GIT_EXE=%%G"
if not defined GIT_EXE (
    echo ERROR: git not found. Install Git for Windows: https://git-scm.com/download/win 1>&2
    exit /b 1
)

rem git.exe lives in <Git>\cmd\ or <Git>\mingw64\bin\ : bash is in <Git>\bin\.
for %%D in ("%GIT_EXE%\..\..") do set "GIT_ROOT=%%~fD"
if not exist "%GIT_ROOT%\bin\bash.exe" for %%D in ("%GIT_EXE%\..\..\..") do set "GIT_ROOT=%%~fD"
if not exist "%GIT_ROOT%\bin\bash.exe" (
    echo ERROR: Git Bash not found next to %GIT_EXE% 1>&2
    exit /b 1
)

"%GIT_ROOT%\bin\bash.exe" "%~dp0dev.sh" %*
exit /b %ERRORLEVEL%
