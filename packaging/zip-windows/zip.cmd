@echo off
setlocal

cd /d "%~dp0..\.."

if not exist "target\packaging\app.exe" (echo zip-windows: target\packaging\app.exe not found - build it first ^(build.cmd^) 1>&2 & exit /b 1)

set ASSET=eso-addons-manager-windows-amd64.zip
set OUT=target\dist\zip-windows

if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%"

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
    "$ErrorActionPreference = 'Stop';" ^
    "Compress-Archive -Path 'target\packaging\app.exe' -DestinationPath '%OUT%\%ASSET%'" || exit /b 1

echo %OUT%\%ASSET%
