using System;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Word = Microsoft.Office.Interop.Word;

namespace MdocWordAddIn
{
    /// <summary>
    /// Cache unter %APPDATA%\MDoc: styles.xml + numbering.xml (Outline) mit mtime+Größe.
    /// </summary>
    internal static class TemplateStylesCache
    {
        private static readonly DateTime UnixEpoch =
            new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);

        internal sealed class TemplateParts
        {
            public string StylesXml;
            public string NumberingXml;
        }

        internal static string ConfigDir
        {
            get { return InstallLayout.ResolveConfigDirectory(); }
        }

        internal static string StylesPath
        {
            get { return Path.Combine(ConfigDir, "template-styles.xml"); }
        }

        internal static string NumberingPath
        {
            get { return Path.Combine(ConfigDir, "template-numbering.xml"); }
        }

        internal static string MetaPath
        {
            get { return Path.Combine(ConfigDir, "template-styles.meta"); }
        }

        internal static string TemplatePathFile
        {
            get { return Path.Combine(ConfigDir, "template-path.txt"); }
        }

        internal static void EnsureFreshBestEffort(Word.Application word)
        {
            try
            {
                string templatePath;
                if (!TryResolveTemplatePath(out templatePath))
                {
                    return;
                }

                if (IsCacheFresh(templatePath))
                {
                    return;
                }

                TemplateParts unused;
                if (TryRefreshFromTemplateFile(templatePath, out unused))
                {
                    return;
                }

                if (word != null)
                {
                    TryRefreshViaWord(word, templatePath, out unused);
                }
            }
            catch
            {
            }
        }

        internal static TemplateParts LoadParts(Word.Application word)
        {
            string templatePath = ResolveTemplatePathOrThrow();

            if (IsCacheFresh(templatePath))
            {
                return ReadCachedParts();
            }

            TemplateParts parts;
            if (TryRefreshFromTemplateFile(templatePath, out parts))
            {
                return parts;
            }

            if (File.Exists(StylesPath))
            {
                try
                {
                    return ReadCachedParts();
                }
                catch
                {
                }
            }

            if (word != null && TryRefreshViaWord(word, templatePath, out parts))
            {
                return parts;
            }

            throw new InvalidOperationException(
                "Die Word-Vorlage ist gesperrt und styles/numbering konnten weder aus der "
                + "Datei noch über Word gelesen werden.\n\n"
                + "Datei: " + templatePath);
        }

        internal static bool TryResolveTemplatePath(out string templatePath)
        {
            templatePath = null;
            string file = TemplatePathFile;
            if (!File.Exists(file))
            {
                return false;
            }

            string line;
            try
            {
                line = File.ReadAllText(file, Encoding.UTF8).Trim();
            }
            catch
            {
                return false;
            }

            using (var reader = new StringReader(line))
            {
                line = (reader.ReadLine() ?? string.Empty).Trim();
            }

            if (string.IsNullOrWhiteSpace(line) || !File.Exists(line))
            {
                return false;
            }

            templatePath = line;
            return true;
        }

        internal static string ResolveTemplatePathOrThrow()
        {
            string path;
            if (!TryResolveTemplatePath(out path))
            {
                if (!File.Exists(TemplatePathFile))
                {
                    throw new InvalidOperationException(
                        "Keine Word-Vorlage hinterlegt.\n\n"
                        + "Bitte im MDoc-Editor eine .dotx/.dotm wählen und das "
                        + "Formatvorlagen-Mapping speichern.");
                }

                throw new InvalidOperationException(
                    "Hinterlegte Vorlage nicht gefunden oder ungültig.\n\n"
                    + "Bitte im MDoc-Editor die Vorlage erneut wählen und das Mapping speichern.");
            }

            return path;
        }

        private static TemplateParts ReadCachedParts()
        {
            var parts = new TemplateParts();
            parts.StylesXml = File.ReadAllText(StylesPath, Encoding.UTF8);
            if (File.Exists(NumberingPath))
            {
                parts.NumberingXml = File.ReadAllText(NumberingPath, Encoding.UTF8);
            }

            return parts;
        }

        private static bool IsCacheFresh(string templatePath)
        {
            if (!File.Exists(StylesPath) || !File.Exists(MetaPath))
            {
                return false;
            }

            Fingerprint cached;
            if (!TryReadMeta(out cached))
            {
                return false;
            }

            Fingerprint current;
            if (!TryFingerprint(templatePath, out current))
            {
                return false;
            }

            if (!PathsEqual(cached.Path, current.Path)
                || cached.MtimeUnixMs != current.MtimeUnixMs
                || cached.Size != current.Size)
            {
                return false;
            }

            // Ältere Caches ohne numbering einmalig erneuern.
            return cached.NumberingPresent == File.Exists(NumberingPath);
        }

        private static bool TryRefreshFromTemplateFile(
            string templatePath,
            out TemplateParts parts)
        {
            parts = null;
            try
            {
                string styles;
                string numbering;
                DocxFlatOpc.ReadTemplatePartsFromDocx(templatePath, out styles, out numbering);
                parts = new TemplateParts
                {
                    StylesXml = styles,
                    NumberingXml = numbering
                };
                PersistCache(templatePath, parts);
                return true;
            }
            catch (IOException)
            {
                return false;
            }
            catch (InvalidOperationException)
            {
                return false;
            }
        }

        private static bool TryRefreshViaWord(
            Word.Application word,
            string templatePath,
            out TemplateParts parts)
        {
            parts = null;
            Word.Document doc = null;
            string tempDir = null;
            var screenUpdating = word.ScreenUpdating;
            try
            {
                word.ScreenUpdating = false;
                tempDir = Path.Combine(
                    Path.GetTempPath(),
                    "mdoc-styles-" + Guid.NewGuid().ToString("N"));
                Directory.CreateDirectory(tempDir);
                string tempDocx = Path.Combine(tempDir, "styles-source.docx");

                object missing = Type.Missing;
                object visible = false;
                object template = templatePath;
                doc = word.Documents.Add(ref template, ref missing, ref missing, ref visible);

                object savePath = tempDocx;
                object format = Word.WdSaveFormat.wdFormatXMLDocument;
                doc.SaveAs(
                    ref savePath,
                    ref format,
                    ref missing, ref missing, ref missing, ref missing, ref missing, ref missing,
                    ref missing, ref missing, ref missing, ref missing, ref missing, ref missing,
                    ref missing, ref missing);

                CloseDocument(doc);
                Marshal.ReleaseComObject(doc);
                doc = null;

                string styles;
                string numbering;
                DocxFlatOpc.ReadTemplatePartsFromDocx(tempDocx, out styles, out numbering);
                parts = new TemplateParts
                {
                    StylesXml = styles,
                    NumberingXml = numbering
                };
                PersistCache(templatePath, parts);
                return true;
            }
            catch
            {
                return false;
            }
            finally
            {
                try { word.ScreenUpdating = screenUpdating; } catch { }
                if (doc != null)
                {
                    try { CloseDocument(doc); } catch { }
                    try { Marshal.ReleaseComObject(doc); } catch { }
                }
                if (tempDir != null)
                {
                    try
                    {
                        if (Directory.Exists(tempDir))
                        {
                            Directory.Delete(tempDir, true);
                        }
                    }
                    catch
                    {
                    }
                }
            }
        }

        private static void PersistCache(string templatePath, TemplateParts parts)
        {
            try
            {
                Directory.CreateDirectory(ConfigDir);
                File.WriteAllText(StylesPath, parts.StylesXml, new UTF8Encoding(false));
                bool numberingPresent = !string.IsNullOrEmpty(parts.NumberingXml);
                if (numberingPresent)
                {
                    File.WriteAllText(NumberingPath, parts.NumberingXml, new UTF8Encoding(false));
                }
                else if (File.Exists(NumberingPath))
                {
                    File.Delete(NumberingPath);
                }

                Fingerprint fp;
                if (TryFingerprint(templatePath, out fp))
                {
                    fp.NumberingPresent = numberingPresent;
                    WriteMeta(fp);
                }
            }
            catch
            {
            }
        }

        private static void CloseDocument(Word.Document document)
        {
            object saveChanges = false;
            object originalFormat = Type.Missing;
            object routeDocument = Type.Missing;
            ((Word._Document)document).Close(
                ref saveChanges, ref originalFormat, ref routeDocument);
        }

        private static bool TryFingerprint(string templatePath, out Fingerprint fingerprint)
        {
            fingerprint = default(Fingerprint);
            try
            {
                var info = new FileInfo(templatePath);
                fingerprint = new Fingerprint
                {
                    Path = templatePath,
                    MtimeUnixMs = ToUnixTimeMilliseconds(info.LastWriteTimeUtc),
                    Size = info.Length,
                    NumberingPresent = false
                };
                return true;
            }
            catch
            {
                return false;
            }
        }

        private static bool TryReadMeta(out Fingerprint fingerprint)
        {
            fingerprint = default(Fingerprint);
            try
            {
                string text = File.ReadAllText(MetaPath, Encoding.UTF8);
                string path = null;
                long? mtime = null;
                long? size = null;
                bool numberingPresent = false;
                bool numberingKeySeen = false;
                using (var reader = new StringReader(text))
                {
                    string line;
                    while ((line = reader.ReadLine()) != null)
                    {
                        line = line.Trim();
                        if (line.Length == 0 || line[0] == '#')
                        {
                            continue;
                        }
                        if (line.StartsWith("path=", StringComparison.Ordinal))
                        {
                            path = line.Substring(5).Trim();
                        }
                        else if (line.StartsWith("mtime_unix_ms=", StringComparison.Ordinal))
                        {
                            long value;
                            if (long.TryParse(
                                line.Substring(14).Trim(),
                                NumberStyles.Integer,
                                CultureInfo.InvariantCulture,
                                out value))
                            {
                                mtime = value;
                            }
                        }
                        else if (line.StartsWith("size=", StringComparison.Ordinal))
                        {
                            long value;
                            if (long.TryParse(
                                line.Substring(5).Trim(),
                                NumberStyles.Integer,
                                CultureInfo.InvariantCulture,
                                out value))
                            {
                                size = value;
                            }
                        }
                        else if (line.StartsWith("numbering=", StringComparison.Ordinal))
                        {
                            numberingKeySeen = true;
                            string value = line.Substring(10).Trim();
                            numberingPresent = value == "1"
                                || string.Equals(value, "true", StringComparison.OrdinalIgnoreCase);
                        }
                    }
                }

                if (path == null || !mtime.HasValue || !size.HasValue)
                {
                    return false;
                }

                // Alte Meta ohne numbering=-Key → Cache als veraltet behandeln.
                if (!numberingKeySeen)
                {
                    return false;
                }

                fingerprint = new Fingerprint
                {
                    Path = path,
                    MtimeUnixMs = mtime.Value,
                    Size = size.Value,
                    NumberingPresent = numberingPresent
                };
                return true;
            }
            catch
            {
                return false;
            }
        }

        private static void WriteMeta(Fingerprint fingerprint)
        {
            string body = string.Format(
                CultureInfo.InvariantCulture,
                "path={0}\nmtime_unix_ms={1}\nsize={2}\nnumbering={3}\n",
                fingerprint.Path,
                fingerprint.MtimeUnixMs,
                fingerprint.Size,
                fingerprint.NumberingPresent ? "1" : "0");
            File.WriteAllText(MetaPath, body, new UTF8Encoding(false));
        }

        private static long ToUnixTimeMilliseconds(DateTime utc)
        {
            return (long)(utc.ToUniversalTime() - UnixEpoch).TotalMilliseconds;
        }

        private static bool PathsEqual(string a, string b)
        {
            if (string.Equals(a, b, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
            if (a == null || b == null)
            {
                return false;
            }
            string na = a.Replace('/', '\\');
            string nb = b.Replace('/', '\\');
            return string.Equals(na, nb, StringComparison.OrdinalIgnoreCase);
        }

        private struct Fingerprint
        {
            public string Path;
            public long MtimeUnixMs;
            public long Size;
            public bool NumberingPresent;
        }
    }
}
