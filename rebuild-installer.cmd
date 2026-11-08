@echo off
setlocal EnableExtensions
cd /d "%~dp0"

echo === 1/2 Build Word add-in ===
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0apps\mdoc-word-addin\build.ps1"
if errorlevel 1 (
  echo ERROR: build.ps1 failed
  exit /b 1
)

echo.
echo === 2/2 Build Tauri NSIS installer ===
cd /d "%~dp0apps\mdoc-desktop"
call npm run tauri -- build
if errorlevel 1 (
  echo ERROR: tauri build failed
  exit /b 1
)

echo.
echo Done. Installer:
set "NSIS=%~dp0target\release\bundle\nsis"
if exist "%NSIS%\*x64-setup.exe" (
  dir /b "%NSIS%\*x64-setup.exe"
  echo Path: %NSIS%
) else (
  echo WARNING: No setup.exe found under %NSIS%
)
echo.
exit /b 0
