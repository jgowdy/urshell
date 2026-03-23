@echo off
REM URShell installer for Windows

set SCRIPT_DIR=%~dp0
set BINARY=%SCRIPT_DIR%native-host\windows-x64\urshell-host.exe

if not exist "%BINARY%" (
    echo Binary not found: %BINARY%
    exit /b 1
)

"%BINARY%" install
