@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0launchers\Start-Windows.ps1"
if errorlevel 1 (
  echo Launch failed. Keep this message for diagnostics. No administrator rights are needed.
  pause
)
