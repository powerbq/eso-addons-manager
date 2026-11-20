@echo off
setlocal

cd /d "%~dp0"

powershell -NoProfile -ExecutionPolicy Bypass -File patches\cargo-patched.ps1 build --profile packaging || goto :failed

echo target\packaging\app.exe
if not defined CI pause
exit /b 0

:failed
if not defined CI pause
exit /b 1
