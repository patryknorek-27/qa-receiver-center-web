@echo off
setlocal
cd /d "%~dp0"

where npm.cmd >nul 2>&1
if errorlevel 1 (
  echo Node.js and npm are required to run QA Receiver Center.
  echo Install Node.js 22 LTS and try again.
  pause
  exit /b 1
)

where cargo.exe >nul 2>&1
if errorlevel 1 (
  echo Rust and Cargo are required to run QA Receiver Center.
  echo Install Rust with rustup and try again.
  pause
  exit /b 1
)

if not exist "node_modules\" (
  echo Installing project dependencies...
  call npm.cmd ci
  if errorlevel 1 goto :error
)

call npm.cmd run tauri:dev
if errorlevel 1 goto :error
exit /b 0

:error
echo.
echo QA Receiver Center could not be started. Review the messages above.
pause
exit /b 1
