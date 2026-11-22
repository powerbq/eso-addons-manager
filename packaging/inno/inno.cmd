@echo off
setlocal

cd /d "%~dp0..\.."

if not exist "target\packaging\app.exe" (echo inno: target\packaging\app.exe not found - build it first ^(build.cmd^) 1>&2 & exit /b 1)

set "VER="
for /f "tokens=3" %%v in ('findstr /b /c:"version = " Cargo.toml') do if not defined VER set "VER=%%~v"

if not defined VER (
    echo could not read version from Cargo.toml 1>&2
    exit /b 1
)

set "ISCC=C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
if exist "%ISCC%" goto :haveiscc
where ISCC >nul 2>&1 && set "ISCC=ISCC" && goto :haveiscc
echo inno: Inno Setup compiler (ISCC) not found - choco install innosetup 1>&2
exit /b 1
:haveiscc

"%ISCC%" "/DMyAppVersion=%VER%" packaging\inno\eso-addons-manager.iss || exit /b 1

echo target\dist\inno\eso-addons-manager-%VER%-windows-amd64-setup.exe
