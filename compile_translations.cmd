@echo off

cd /d "%~dp0"

for %%f in (translations\*.ts) do pyside6-lrelease "%%f" -qm "translations\%%~nf.qm"
