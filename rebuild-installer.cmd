@echo off
setlocal EnableExtensions
cd /d D:\Projekte\mdword

echo === 1/3 Add-in registrieren (HKCU) ===
call apps\mdoc-word-addin\finish-hang-recovery.cmd
if errorlevel 1 echo WARNUNG: Registrierung fehlgeschlagen (DLL fehlt?)

echo.
echo === 2/3 Word-Add-in bundlen ===
powershell -NoProfile -ExecutionPolicy Bypass -File apps\mdoc-word-addin\build.ps1
if errorlevel 1 (
  echo FEHLER: build.ps1
  exit /b 1
)

echo.
echo === 3/3 CLI + Tauri NSIS-Installer ===
cd apps\mdoc-desktop
call npm run bundle:cli
if errorlevel 1 (
  echo FEHLER: bundle:cli
  exit /b 1
)
call npm run tauri -- build
if errorlevel 1 (
  echo FEHLER: tauri build
  exit /b 1
)

echo.
echo Fertig. Neuer Installer:
dir /b ..\..\target\release\bundle\nsis\*x64-setup.exe 2>nul
dir /b src-tauri\target\release\bundle\nsis\*x64-setup.exe 2>nul
echo.
