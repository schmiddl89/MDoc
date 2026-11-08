using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using Extensibility;
using Microsoft.Office.Core;
using Word = Microsoft.Office.Interop.Word;

namespace MdocWordAddIn
{
    /// <summary>
    /// Clean-Room COM-Add-in mit echten Office-/Extensibility-PIAs.
    /// Kein Code aus dem PoC. Fehler werden geschluckt, damit Word startfähig bleibt.
    /// </summary>
    [ComVisible(true)]
    [Guid("B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874")]
    [ProgId("mdoc.Connect")]
    [ClassInterface(ClassInterfaceType.None)]
    [ComDefaultInterface(typeof(IRibbonCallbacks))]
    public class Connect : IDTExtensibility2, IRibbonExtensibility, IRibbonCallbacks
    {
        private Word.Application _word;
        private IRibbonUI _ribbon;
        private static readonly string LogPath = Path.Combine(
            InstallLayout.ResolveConfigDirectory(),
            "word-addin.log");

        public void OnConnection(object application, ext_ConnectMode connectMode, object addInInst, ref Array custom)
        {
            try
            {
                _word = (Word.Application)application;
                Log("OnConnection ok, mode=" + connectMode);
            }
            catch (Exception ex)
            {
                Log("OnConnection failed: " + ex);
                // Word darf trotzdem weiterlaufen; Add-in bleibt geladen ohne Word-Ref.
                _word = null;
            }
        }

        public void OnDisconnection(ext_DisconnectMode removeMode, ref Array custom)
        {
            try
            {
                _ribbon = null;
                if (_word != null)
                {
                    try { Marshal.ReleaseComObject(_word); } catch { }
                    _word = null;
                }
            }
            catch (Exception ex)
            {
                Log("OnDisconnection: " + ex.Message);
            }
        }

        public void OnAddInsUpdate(ref Array custom)
        {
        }

        public void OnStartupComplete(ref Array custom)
        {
            Log("OnStartupComplete");
            // mtime+Größe: nur bei geänderter Vorlage styles.xml nachladen.
            TemplateStylesCache.EnsureFreshBestEffort(_word);
        }

        public void OnBeginShutdown(ref Array custom)
        {
            _ribbon = null;
        }

        public string GetCustomUI(string ribbonID)
        {
            try
            {
                var placement = ReadRibbonPlacement();
                var lang = UiText.Language;
                string resource;
                if (placement == "ownTab")
                {
                    resource = lang == "de"
                        ? "MdocWordAddIn.Ribbon.OwnTab.xml"
                        : "MdocWordAddIn.Ribbon.OwnTab.en.xml";
                }
                else
                {
                    resource = lang == "de"
                        ? "MdocWordAddIn.Ribbon.Home.xml"
                        : "MdocWordAddIn.Ribbon.Home.en.xml";
                }

                var stream = Assembly.GetExecutingAssembly()
                    .GetManifestResourceStream(resource);
                if (stream == null)
                {
                    Log("Ribbon resource missing: " + resource);
                    return string.Empty;
                }

                using (stream)
                using (var reader = new StreamReader(stream))
                {
                    var xml = reader.ReadToEnd();
                    Log("GetCustomUI placement=" + placement + " lang=" + lang);
                    return xml;
                }
            }
            catch (Exception ex)
            {
                Log("GetCustomUI failed: " + ex);
                return string.Empty;
            }
        }

        /// <summary>
        /// Ribbon-Platzierung aus %APPDATA%\MDoc\ribbon-placement:
        ///   home   = Gruppe unter Registerkarte Start (Standard)
        ///   ownTab = eigene Registerkarte „MDoc“
        /// </summary>
        private static string ReadRibbonPlacement()
        {
            try
            {
                var path = Path.Combine(
                    InstallLayout.ResolveConfigDirectory(),
                    "ribbon-placement");
                if (!File.Exists(path))
                {
                    return "home";
                }

                var raw = File.ReadAllText(path).Trim();
                if (raw.Length == 0)
                {
                    return "home";
                }

                // Nur erste Zeile / erstes Token auswerten.
                var token = raw.Split(new[] { '\r', '\n', ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries)[0]
                    .Trim()
                    .ToLowerInvariant();
                if (token == "owntab" || token == "tab" || token == "mdoc" || token == "mdword")
                {
                    return "ownTab";
                }

                return "home";
            }
            catch (Exception ex)
            {
                Log("ReadRibbonPlacement: " + ex.Message);
                return "home";
            }
        }

        public void OnRibbonLoad(IRibbonUI ribbonUi)
        {
            try
            {
                _ribbon = ribbonUi;
                Log("OnRibbonLoad ok");
            }
            catch (Exception ex)
            {
                Log("OnRibbonLoad failed: " + ex.Message);
            }
        }

        public void OnMapStyles(IRibbonControl control)
        {
            RunSafe(() => LaunchMdoc("--map-styles"));
        }

        public void OnOpenEditor(IRibbonControl control)
        {
            RunSafe(() => LaunchMdoc(null));
        }

        public void OnInsertMarkdown(IRibbonControl control)
        {
            RunSafe(() => WordMarkdownBridge.InsertMarkdownFromClipboard(_word));
        }

        public void OnCopyMarkdown(IRibbonControl control)
        {
            RunSafe(() => WordMarkdownBridge.CopySelectionAsMarkdown(_word));
        }

        private void RunSafe(Action action)
        {
            try
            {
                action();
            }
            catch (Exception ex)
            {
                Log("Action failed: " + ex);
                try
                {
                    System.Windows.Forms.MessageBox.Show(
                        UiText.ActionFailed + ex.Message,
                        "MDoc",
                        System.Windows.Forms.MessageBoxButtons.OK,
                        System.Windows.Forms.MessageBoxIcon.Warning);
                }
                catch
                {
                }
            }
        }

        private static void LaunchMdoc(string arguments)
        {
            var exe = ResolveMdocExe();
            if (exe == null)
            {
                throw new InvalidOperationException(UiText.DesktopExeMissing);
            }

            Process.Start(new ProcessStartInfo
            {
                FileName = exe,
                Arguments = arguments ?? string.Empty,
                UseShellExecute = true,
            });
        }

        private static string ResolveMdocExe()
        {
            return InstallLayout.ResolveDesktopExe();
        }

        private static void Log(string message)
        {
            try
            {
                var dir = Path.GetDirectoryName(LogPath);
                if (!string.IsNullOrEmpty(dir))
                {
                    Directory.CreateDirectory(dir);
                }

                File.AppendAllText(
                    LogPath,
                    DateTime.Now.ToString("s") + " " + message + Environment.NewLine);
            }
            catch
            {
            }
        }
    }
}
