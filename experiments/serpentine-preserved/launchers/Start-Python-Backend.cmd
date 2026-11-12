@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0launchers\Start-Windows.ps1" -PythonBackend
if errorlevel 1 pause
