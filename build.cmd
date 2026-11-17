@echo off
setlocal

cd /d "%~dp0"

set ASSET=eso-addons-manager-windows-amd64.zip

powershell -NoProfile -ExecutionPolicy Bypass -File patches\cargo-patched.ps1 build --profile packaging || goto :failed

if exist target\dist rmdir /s /q target\dist
mkdir target\dist

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
    "$ErrorActionPreference = 'Stop';" ^
    "Compress-Archive -Path 'target\packaging\app.exe' -DestinationPath 'target\dist\%ASSET%';" ^
    "$hash = (Get-FileHash -Algorithm SHA256 'target\dist\%ASSET%').Hash.ToLowerInvariant();" ^
    "[IO.File]::WriteAllText('target\dist\%ASSET%.sha256', $hash)" || goto :failed

echo target\dist\%ASSET%
if not defined CI pause
exit /b 0

:failed
if not defined CI pause
exit /b 1
