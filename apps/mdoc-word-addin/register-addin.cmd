@echo off
setlocal EnableExtensions
REM Schnelle HKCU-Registrierung ohne PowerShell (Installer-sicher).
set "DLL=%~1"
if "%DLL%"=="" set "DLL=%~dp0MdocWordAddIn.dll"
if not exist "%DLL%" (
  echo DLL nicht gefunden: %DLL%
  exit /b 1
)

set "CB=%DLL:\=/%"
set "CB=file:///%CB%"
set "CLSID={B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}"
set "PROGID=mdoc.Connect"
set "ASM=MdocWordAddIn, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"

reg add "HKCU\Software\Classes\CLSID\%CLSID%" /ve /d "%PROGID%" /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32" /ve /d "mscoree.dll" /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32" /v ThreadingModel /d Both /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32" /v Class /d MdocWordAddIn.Connect /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32" /v Assembly /d "%ASM%" /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32" /v RuntimeVersion /d v4.0.30319 /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32" /v CodeBase /d "%CB%" /f >nul

reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32\1.0.0.0" /ve /d "mscoree.dll" /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32\1.0.0.0" /v ThreadingModel /d Both /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32\1.0.0.0" /v Class /d MdocWordAddIn.Connect /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32\1.0.0.0" /v Assembly /d "%ASM%" /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32\1.0.0.0" /v RuntimeVersion /d v4.0.30319 /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\InprocServer32\1.0.0.0" /v CodeBase /d "%CB%" /f >nul

reg add "HKCU\Software\Classes\CLSID\%CLSID%\ProgID" /ve /d "%PROGID%" /f >nul
reg add "HKCU\Software\Classes\CLSID\%CLSID%\Implemented Categories\{62C8FE65-4EBB-45e7-B440-6E39B2CDBF29}" /ve /d "" /f >nul

reg add "HKCU\Software\Classes\%PROGID%" /ve /d "MDoc Word Add-in" /f >nul
reg add "HKCU\Software\Classes\%PROGID%\CLSID" /ve /d "%CLSID%" /f >nul

reg add "HKCU\Software\Microsoft\Office\Word\Addins\%PROGID%" /v Description /d "MDoc Markdown-Integration" /f >nul
reg add "HKCU\Software\Microsoft\Office\Word\Addins\%PROGID%" /v FriendlyName /d "MDoc" /f >nul
reg add "HKCU\Software\Microsoft\Office\Word\Addins\%PROGID%" /v LoadBehavior /t REG_DWORD /d 3 /f >nul

echo MDoc Word-Add-in registriert.
echo CodeBase=%CB%
exit /b 0
