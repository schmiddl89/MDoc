using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows.Forms;
using Word = Microsoft.Office.Interop.Word;

namespace MdocWordAddIn
{
    /// <summary>
    /// Markdown ↔ Word über Temp-DOCX + mdoc-cli.
    /// Einfügen: InsertXML mit styles.xml aus Editor-Vorlage
    /// (mtime+Größe-Cache unter %APPDATA%\MDoc, Share-Read bei Änderung).
    /// </summary>
    internal static class WordMarkdownBridge
    {
        public static void InsertMarkdownFromClipboard(Word.Application word)
        {
            if (word == null)
            {
                throw new InvalidOperationException("Word-Anwendung ist nicht verfügbar.");
            }

            var markdown = Clipboard.GetText(TextDataFormat.UnicodeText);
            if (string.IsNullOrWhiteSpace(markdown))
            {
                markdown = Clipboard.GetText(TextDataFormat.Text);
            }
            if (string.IsNullOrWhiteSpace(markdown))
            {
                throw new InvalidOperationException(
                    "Zwischenablage enthält keinen Text (Markdown erwartet).");
            }

            if (word.ActiveDocument == null)
            {
                throw new InvalidOperationException("Kein aktives Word-Dokument.");
            }

            var templateParts = TemplateStylesCache.LoadParts(word);

            var workDir = CreateWorkDir();
            var mdPath = Path.Combine(workDir, "paste.md");
            var docxPath = Path.Combine(workDir, "paste.docx");
            var screenUpdating = word.ScreenUpdating;
            try
            {
                File.WriteAllText(mdPath, markdown, new UTF8Encoding(false));
                MdocCli.ConvertMarkdownToDocx(mdPath, docxPath);
                if (!File.Exists(docxPath))
                {
                    throw new InvalidOperationException("DOCX-Konvertierung lieferte keine Datei.");
                }

                string flatOpc = DocxFlatOpc.FromDocxFile(
                    docxPath,
                    templateParts.StylesXml,
                    templateParts.NumberingXml);

                word.ScreenUpdating = false;
                InsertFlatOpc(word, flatOpc);
            }
            finally
            {
                try { word.ScreenUpdating = screenUpdating; } catch { }
                TryDeleteDir(workDir);
            }
        }

        public static void CopySelectionAsMarkdown(Word.Application word)
        {
            if (word == null)
            {
                throw new InvalidOperationException("Word-Anwendung ist nicht verfügbar.");
            }

            Word.Selection selection = word.Selection;
            if (selection == null || selection.Range == null || selection.Range.Start == selection.Range.End)
            {
                throw new InvalidOperationException("Bitte zuerst Text in Word markieren.");
            }

            var workDir = CreateWorkDir();
            var docxPath = Path.Combine(workDir, "selection.docx");
            var mdPath = Path.Combine(workDir, "selection.md");
            Word.Document tempDoc = null;
            var screenUpdating = word.ScreenUpdating;
            try
            {
                word.ScreenUpdating = false;
                object visible = false;
                object missing = Missing();
                tempDoc = word.Documents.Add(ref missing, ref missing, ref missing, ref visible);
                tempDoc.Content.FormattedText = selection.Range.FormattedText;

                object savePath = docxPath;
                object format = Word.WdSaveFormat.wdFormatXMLDocument;
                tempDoc.SaveAs(
                    ref savePath,
                    ref format,
                    ref missing, ref missing, ref missing, ref missing, ref missing, ref missing,
                    ref missing, ref missing, ref missing, ref missing, ref missing, ref missing,
                    ref missing, ref missing);

                CloseDocument(tempDoc);
                Marshal.ReleaseComObject(tempDoc);
                tempDoc = null;

                MdocCli.ImportDocxToMarkdown(docxPath, mdPath);
                var markdown = File.ReadAllText(mdPath, Encoding.UTF8);
                Clipboard.SetText(markdown ?? string.Empty, TextDataFormat.UnicodeText);
            }
            finally
            {
                try { word.ScreenUpdating = screenUpdating; } catch { }
                if (tempDoc != null)
                {
                    try { CloseDocument(tempDoc); } catch { }
                    try { Marshal.ReleaseComObject(tempDoc); } catch { }
                }
                TryDeleteDir(workDir);
            }
        }

        private static void InsertFlatOpc(Word.Application word, string flatOpc)
        {
            Word.Selection selection = word.Selection;
            if (selection == null || selection.Range == null)
            {
                throw new InvalidOperationException("Keine Word-Auswahl zum Einfügen.");
            }

            try
            {
                selection.Range.InsertXML(flatOpc);
            }
            catch (COMException ex)
            {
                throw new InvalidOperationException(
                    "InsertXML fehlgeschlagen: " + ex.Message, ex);
            }
        }

        private static void CloseDocument(Word.Document document)
        {
            object saveChanges = false;
            object originalFormat = Missing();
            object routeDocument = Missing();
            ((Word._Document)document).Close(ref saveChanges, ref originalFormat, ref routeDocument);
        }

        private static string CreateWorkDir()
        {
            var dir = Path.Combine(
                Path.GetTempPath(),
                "mdoc-word-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(dir);
            return dir;
        }

        private static void TryDeleteDir(string dir)
        {
            try
            {
                if (Directory.Exists(dir))
                {
                    Directory.Delete(dir, true);
                }
            }
            catch
            {
            }
        }

        private static object Missing()
        {
            return Type.Missing;
        }
    }
}
