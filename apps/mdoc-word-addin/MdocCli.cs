using System;
using System.Diagnostics;
using System.IO;
using System.Text;

namespace MdocWordAddIn
{
    /// <summary>
    /// Ruft mdoc-cli auf (gemeinsamer Rust-Kern). Keine eigene Markdown-/DOCX-Logik.
    /// </summary>
    internal static class MdocCli
    {
        public static void ConvertMarkdownToDocx(string markdownPath, string docxPath)
        {
            Run("convert", Quote(markdownPath), Quote(docxPath), "--force");
        }

        public static void ImportDocxToMarkdown(string docxPath, string markdownPath)
        {
            Run("import", Quote(docxPath), Quote(markdownPath), "--force");
        }

        private static void Run(string command, params string[] args)
        {
            var exe = ResolveCliExe();
            if (exe == null)
            {
                throw new InvalidOperationException(
                    "mdoc-cli.exe wurde nicht gefunden. Bitte MDoc erneut installieren.");
            }

            var arguments = new StringBuilder(command);
            foreach (var arg in args)
            {
                arguments.Append(' ').Append(arg);
            }

            // convert: Style-Profil + ggf. Normal.dotm (Formatvorlagen im Zieldokument).
            if (string.Equals(command, "convert", StringComparison.OrdinalIgnoreCase)
                || string.Equals(command, "import", StringComparison.OrdinalIgnoreCase))
            {
                var profile = SharedStyleProfilePath();
                if (profile != null)
                {
                    arguments.Append(" --style-profile ").Append(Quote(profile));
                }
            }
            // Kein Normal.dotm als --template: die Datei enthält oft nur wenige Styles und
            // würde die Heading-Definitionen (heading 1 ↔ Überschrift) beim Einfügen verdrängen.
            // Style-IDs kommen aus dem gemeinsamen Profil; Definitionen schreibt mdoc-core.

            var start = new ProcessStartInfo
            {
                FileName = exe,
                Arguments = arguments.ToString(),
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                StandardOutputEncoding = Encoding.UTF8,
                StandardErrorEncoding = Encoding.UTF8,
            };

            using (var process = Process.Start(start))
            {
                if (process == null)
                {
                    throw new InvalidOperationException("mdoc-cli konnte nicht gestartet werden.");
                }

                var stdout = process.StandardOutput.ReadToEnd();
                var stderr = process.StandardError.ReadToEnd();
                process.WaitForExit(120000);
                if (!process.HasExited)
                {
                    try { process.Kill(); } catch { }
                    throw new TimeoutException("mdoc-cli hat zu lange gebraucht.");
                }

                if (process.ExitCode != 0)
                {
                    var detail = string.IsNullOrWhiteSpace(stderr) ? stdout : stderr;
                    throw new InvalidOperationException(
                        "mdoc-cli fehlgeschlagen (Exit " + process.ExitCode + "): " + detail.Trim());
                }
            }
        }

        private static string Quote(string path)
        {
            return "\"" + path + "\"";
        }

        private static string SharedStyleProfilePath()
        {
            var path = Path.Combine(
                InstallLayout.ResolveConfigDirectory(),
                "style-profile.json");
            return File.Exists(path) ? path : null;
        }

        public static string ResolveCliExe()
        {
            return InstallLayout.ResolveCliExe();
        }
    }
}
