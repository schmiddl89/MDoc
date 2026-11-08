#Requires -Version 5.1
param(
  [string]$DllPath = $null,
  [switch]$Unregister
)

$ErrorActionPreference = 'Stop'
$clsid = '{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}'
$progId = 'mdoc.Connect'
$legacyProgId = 'mdword.Connect'
$assemblyName = 'MdocWordAddIn, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null'
$className = 'MdocWordAddIn.Connect'
$netCategory = '{62C8FE65-4EBB-45e7-B440-6E39B2CDBF29}'

function Get-DefaultDllPath {
  foreach ($folder in @('MDoc', 'mdoc', 'mdword')) {
    $local = Join-Path $env:LOCALAPPDATA "$folder\word-addin\MdocWordAddIn.dll"
    if (Test-Path $local) { return (Resolve-Path $local).Path }
  }
  $beside = Join-Path $PSScriptRoot 'bin\Release\MdocWordAddIn.dll'
  if (Test-Path $beside) { return (Resolve-Path $beside).Path }
  $bundle = Join-Path $PSScriptRoot '..\mdoc-desktop\src-tauri\word-addin\MdocWordAddIn.dll'
  if (Test-Path $bundle) { return (Resolve-Path $bundle).Path }
  throw 'MdocWordAddIn.dll nicht gefunden.'
}

function ConvertTo-FileUri([string]$path) {
  $full = (Resolve-Path $path).Path
  return ('file:///' + ($full -replace '\\', '/'))
}

function Remove-MdocRegistration {
  Remove-Item -Path "HKCU:\Software\Microsoft\Office\Word\Addins\$progId" -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item -Path "HKCU:\Software\Classes\$progId" -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item -Path "HKCU:\Software\Microsoft\Office\Word\Addins\$legacyProgId" -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item -Path "HKCU:\Software\Classes\$legacyProgId" -Recurse -Force -ErrorAction SilentlyContinue
  Remove-Item -Path "HKCU:\Software\Classes\CLSID\$clsid" -Recurse -Force -ErrorAction SilentlyContinue

  Get-ChildItem 'HKCU:\Software\Microsoft\Office' -ErrorAction SilentlyContinue | ForEach-Object {
    $disabled = Join-Path $_.PSPath 'Word\Resiliency\DisabledItems'
    if (Test-Path $disabled) {
      Get-ItemProperty $disabled -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty PSObject -ErrorAction SilentlyContinue | Out-Null
      # Binary disabled blobs cannot be reliably filtered; clear known empty tree only if sole purpose is mdoc.
    }
  }
}

if ($Unregister) {
  Remove-MdocRegistration
  Write-Host 'MDoc Word-Add-in deregistriert (HKCU).'
  exit 0
}

if (-not $DllPath) { $DllPath = Get-DefaultDllPath }
if (-not (Test-Path $DllPath)) { throw "DLL fehlt: $DllPath" }

$codeBase = ConvertTo-FileUri $DllPath
Remove-MdocRegistration

New-Item -Path "HKCU:\Software\Classes\CLSID\$clsid" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Classes\CLSID\$clsid" -Name '(default)' -Value $progId

$inproc = "HKCU:\Software\Classes\CLSID\$clsid\InprocServer32"
New-Item -Path $inproc -Force | Out-Null
Set-ItemProperty -Path $inproc -Name '(default)' -Value 'mscoree.dll'
New-ItemProperty -Path $inproc -Name 'ThreadingModel' -Value 'Both' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inproc -Name 'Class' -Value $className -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inproc -Name 'Assembly' -Value $assemblyName -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inproc -Name 'RuntimeVersion' -Value 'v4.0.30319' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inproc -Name 'CodeBase' -Value $codeBase -PropertyType String -Force | Out-Null

$inprocVer = "$inproc\1.0.0.0"
New-Item -Path $inprocVer -Force | Out-Null
Set-ItemProperty -Path $inprocVer -Name '(default)' -Value 'mscoree.dll'
New-ItemProperty -Path $inprocVer -Name 'ThreadingModel' -Value 'Both' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inprocVer -Name 'Class' -Value $className -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inprocVer -Name 'Assembly' -Value $assemblyName -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inprocVer -Name 'RuntimeVersion' -Value 'v4.0.30319' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $inprocVer -Name 'CodeBase' -Value $codeBase -PropertyType String -Force | Out-Null

New-Item -Path "HKCU:\Software\Classes\CLSID\$clsid\ProgID" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Classes\CLSID\$clsid\ProgID" -Name '(default)' -Value $progId

New-Item -Path "HKCU:\Software\Classes\CLSID\$clsid\Implemented Categories\$netCategory" -Force | Out-Null

New-Item -Path "HKCU:\Software\Classes\$progId" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Classes\$progId" -Name '(default)' -Value 'MDoc Word Add-in'
New-Item -Path "HKCU:\Software\Classes\$progId\CLSID" -Force | Out-Null
Set-ItemProperty -Path "HKCU:\Software\Classes\$progId\CLSID" -Name '(default)' -Value $clsid

$addin = "HKCU:\Software\Microsoft\Office\Word\Addins\$progId"
New-Item -Path $addin -Force | Out-Null
New-ItemProperty -Path $addin -Name 'Description' -Value 'MDoc Markdown-Integration' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $addin -Name 'FriendlyName' -Value 'MDoc' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $addin -Name 'LoadBehavior' -Value 3 -PropertyType DWord -Force | Out-Null

# Soft-disable-Einträge von Word entfernen (alle Office-Versionen).
Get-ChildItem 'HKCU:\Software\Microsoft\Office' -ErrorAction SilentlyContinue | ForEach-Object {
  $resiliency = Join-Path $_.PSPath 'Word\Resiliency'
  if (Test-Path $resiliency) {
    Remove-Item -Path (Join-Path $resiliency 'DisabledItems') -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path (Join-Path $resiliency 'DoNotDisableAddinList') -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Write-Host "MDoc Word-Add-in registriert."
Write-Host "  DLL:      $DllPath"
Write-Host "  CodeBase: $codeBase"
Write-Host "  LoadBehavior=3 - bitte Word vollstaendig beenden und neu starten."
