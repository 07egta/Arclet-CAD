@echo off
setlocal
set "SCRIPT=%~dp0Start-Windows.ps1"
if not exist "%SCRIPT%" set "SCRIPT=%~dp0launchers\Start-Windows.ps1"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT%" -PythonBackend %*
if errorlevel 1 (
  echo Python backend launch failed.
  pause
  exit /b 1
)
