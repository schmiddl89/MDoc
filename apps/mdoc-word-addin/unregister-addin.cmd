@echo off
setlocal EnableExtensions
set "CLSID={B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}"
set "PROGID=mdoc.Connect"
set "LEGACY=mdword.Connect"
reg delete "HKCU\Software\Microsoft\Office\Word\Addins\%PROGID%" /f >nul 2>&1
reg delete "HKCU\Software\Classes\%PROGID%" /f >nul 2>&1
reg delete "HKCU\Software\Microsoft\Office\Word\Addins\%LEGACY%" /f >nul 2>&1
reg delete "HKCU\Software\Classes\%LEGACY%" /f >nul 2>&1
reg delete "HKCU\Software\Classes\CLSID\%CLSID%" /f >nul 2>&1
echo MDoc Word-Add-in deregistriert.
exit /b 0
