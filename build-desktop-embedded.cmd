@echo off
setlocal
set "RUSTUP_HOME=D:\build-tools\rustup"
set "CARGO_HOME=D:\build-tools\cargo"
set "PATH=D:\build-tools\cargo\bin;%PATH%"
set "CARGO_INCREMENTAL=0"
set "CARGO_BUILD_JOBS=2"
set "CARGO_PROFILE_DEV_DEBUG=0"
set "CARGO_PROFILE_DEV_CODEGEN_UNITS=1"

echo === Building Frontend ===
cd /d D:\ProxyWeave\apps\desktop
call npm run build
if errorlevel 1 (
  echo Frontend build failed!
  exit /b 1
)

echo === Building Tauri Desktop Embedded Binary ===
call npm run tauri -- build --debug --no-bundle
if errorlevel 1 (
  echo Tauri build failed!
  exit /b 2
)

echo === Build Successful ===
