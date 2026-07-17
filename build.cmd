@echo off

call "%~dp0compile_translations.cmd" || exit /b 1

if not exist build mkdir build

cd build

pyinstaller --clean --distpath dist --workpath work ..\app.spec

pause
