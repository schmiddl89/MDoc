@echo off
setlocal EnableExtensions
REM Nach haengendem Installer: Setup-Reste beenden und Add-in registrieren.
taskkill /F /IM MDoc_1.0.0_x64-setup.exe /T >nul 2>&1
taskkill /F /IM "MDoc_1.0.0_x64-setup.tmp" /T >nul 2>&1
taskkill /F /IM mdword_1.0.0_x64-setup.exe /T >nul 2>&1
taskkill /F /IM "mdword_1.0.0_x64-setup.tmp" /T >nul 2>&1

set "DLL="
for %%F in (MDoc mdoc mdword) do (
  if exist "%LOCALAPPDATA%\%%F\word-addin\MdocWordAddIn.dll" set "DLL=%LOCALAPPDATA%\%%F\word-addin\MdocWordAddIn.dll"
)
if "%DLL%"=="" (
  echo DLL fehlt unter %%LOCALAPPDATA%%\MDoc , mdoc oder mdword\word-addin\
  exit /b 1
)

for %%F in (MDoc mdoc mdword) do (
  if exist "%LOCALAPPDATA%\%%F\word-addin\" (
    copy /Y "%~dp0register-addin.cmd" "%LOCALAPPDATA%\%%F\word-addin\" >nul
    copy /Y "%~dp0unregister-addin.cmd" "%LOCALAPPDATA%\%%F\word-addin\" >nul
  )
)
call "%~dp0register-addin.cmd" "%DLL%"
exit /b %ERRORLEVEL%
