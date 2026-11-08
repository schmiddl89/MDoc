$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$outDir = Join-Path $root 'bin\Release'
$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if (-not (Test-Path $csc)) { throw "csc.exe not found: $csc" }

$ext = Join-Path $root 'lib\Extensibility.dll'
$office = Join-Path $root 'lib\Office.dll'
$word = Join-Path $root 'packages\Microsoft.Office.Interop.Word.15.0.4797.1004\lib\net20\Microsoft.Office.Interop.Word.dll'
foreach ($req in @($ext, $office, $word)) {
  if (-not (Test-Path $req)) { throw "Missing reference: $req" }
}

New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$sources = @(
  (Join-Path $root 'Connect.cs'),
  (Join-Path $root 'IRibbonCallbacks.cs'),
  (Join-Path $root 'MdocCli.cs'),
  (Join-Path $root 'DocxFlatOpc.cs'),
  (Join-Path $root 'TemplateStylesCache.cs'),
  (Join-Path $root 'InstallLayout.cs'),
  (Join-Path $root 'AppSettings.cs'),
  (Join-Path $root 'UiText.cs'),
  (Join-Path $root 'WordMarkdownBridge.cs'),
  (Join-Path $root 'Properties\AssemblyInfo.cs')
)
$netFx = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319'
$compression = Join-Path $netFx 'System.IO.Compression.dll'
$compressionFs = Join-Path $netFx 'System.IO.Compression.FileSystem.dll'
$systemXml = Join-Path $netFx 'System.Xml.dll'
foreach ($req in @($compression, $compressionFs, $systemXml)) {
  if (-not (Test-Path $req)) { throw "Missing reference: $req" }
}
$ribbonHome = Join-Path $root 'Ribbon.Home.xml'
$ribbonOwnTab = Join-Path $root 'Ribbon.OwnTab.xml'
$ribbonHomeEn = Join-Path $root 'Ribbon.Home.en.xml'
$ribbonOwnTabEn = Join-Path $root 'Ribbon.OwnTab.en.xml'
$dll = Join-Path $outDir 'MdocWordAddIn.dll'

# /link embeds COM-Interop-Typen (wie EmbedInteropTypes), damit keine Extra-PIAs zur Laufzeit noetig sind.
& $csc /nologo /target:library /optimize+ /platform:anycpu `
  /reference:System.dll /reference:System.Core.dll /reference:System.Windows.Forms.dll `
  /reference:$systemXml /reference:$compression /reference:$compressionFs `
  /link:$ext /link:$office /link:$word `
  /resource:"$ribbonHome,MdocWordAddIn.Ribbon.Home.xml" `
  /resource:"$ribbonOwnTab,MdocWordAddIn.Ribbon.OwnTab.xml" `
  /resource:"$ribbonHomeEn,MdocWordAddIn.Ribbon.Home.en.xml" `
  /resource:"$ribbonOwnTabEn,MdocWordAddIn.Ribbon.OwnTab.en.xml" `
  /out:$dll `
  $sources

if ($LASTEXITCODE -ne 0) { throw "csc failed: $LASTEXITCODE" }

$bundle = Join-Path $root '..\mdoc-desktop\src-tauri\word-addin'
New-Item -ItemType Directory -Force -Path $bundle | Out-Null
Copy-Item $dll (Join-Path $bundle 'MdocWordAddIn.dll') -Force
Copy-Item (Join-Path $root 'README.md') (Join-Path $bundle 'README.md') -Force
Copy-Item (Join-Path $root 'Register-MdocAddIn.ps1') (Join-Path $bundle 'Register-MdocAddIn.ps1') -Force
Copy-Item (Join-Path $root 'register-addin.cmd') (Join-Path $bundle 'register-addin.cmd') -Force
Copy-Item (Join-Path $root 'unregister-addin.cmd') (Join-Path $bundle 'unregister-addin.cmd') -Force
Write-Host "Built and copied to $bundle"

# Laufzeit-Kopien: MDoc (Installer), mdoc und legacy mdword
foreach ($folder in @('MDoc', 'mdoc', 'mdword')) {
  $installed = Join-Path $env:LOCALAPPDATA "$folder\word-addin"
  if (-not (Test-Path (Split-Path $installed -Parent))) { continue }
  New-Item -ItemType Directory -Force -Path $installed | Out-Null
  try {
    Copy-Item $dll (Join-Path $installed 'MdocWordAddIn.dll') -Force
    Write-Host "Copied to $installed"
  } catch {
    Write-Warning "Could not copy to $installed (Word locking the DLL?). Quit Word and rebuild/register. $_"
  }
}
