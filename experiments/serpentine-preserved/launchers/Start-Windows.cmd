@echo off
setlocal
set "SCRIPT=%~dp0Start-Windows.ps1"
if not exist "%SCRIPT%" set "SCRIPT=%~dp0launchers\Start-Windows.ps1"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" %*
if errorlevel 1 (
  echo Launch failed. Read the diagnostic above. Administrator rights are not needed.
  pause
  exit /b 1
)
