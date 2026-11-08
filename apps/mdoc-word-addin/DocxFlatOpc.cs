using System;
using System.Globalization;
using System.IO;
using System.IO.Compression;
using System.Text;
using System.Text.RegularExpressions;
using System.Xml;

namespace MdocWordAddIn
{
    /// <summary>
    /// Flat-OPC für Range.InsertXML: Inhalt aus der generierten DOCX, Formatvorlagen
    /// und Outline-Nummerierung aus der Editor-Vorlage; Listen-Defs (numId &gt;= 1000)
    /// aus der generierten DOCX werden angehängt — wie im Rust-Export.
    /// </summary>
    internal static class DocxFlatOpc
    {
        private const string PkgNs = "http://schemas.microsoft.com/office/2006/xmlPackage";
        private const string RelNs = "http://schemas.openxmlformats.org/package/2006/relationships";
        private const string OfficeRelNs =
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

        /// <summary>Gleicher Sockel wie mdoc-core LIST_NUMBERING_ID_BASE.</summary>
        private const int ListNumberingIdBase = 1000;

        public static string FromDocxFile(
            string docxPath,
            string templateStylesXml,
            string templateNumberingXml)
        {
            if (string.IsNullOrWhiteSpace(templateStylesXml))
            {
                throw new InvalidOperationException(
                    "styles.xml der Vorlage fehlt — Einfügen abgebrochen.");
            }

            string documentXml;
            string generatedNumberingXml = null;
            string documentRelsXml = null;

            using (var zip = ZipFile.OpenRead(docxPath))
            {
                documentXml = ReadEntryText(zip, "word/document.xml");
                if (documentXml == null)
                {
                    throw new InvalidOperationException("DOCX enthält kein word/document.xml.");
                }

                generatedNumberingXml = ReadEntryText(zip, "word/numbering.xml");
                documentRelsXml = ReadEntryText(zip, "word/_rels/document.xml.rels");
            }

            string numberingXml = MergeNumbering(templateNumberingXml, generatedNumberingXml);
            documentRelsXml = BuildDocumentRels(documentRelsXml, numberingXml != null);

            var sb = new StringBuilder(
                documentXml.Length
                + templateStylesXml.Length
                + (numberingXml != null ? numberingXml.Length : 0)
                + 4096);
            sb.Append("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>");
            sb.Append("<?mso-application progid=\"Word.Document\"?>");
            sb.Append("<pkg:package xmlns:pkg=\"").Append(PkgNs).Append("\">");

            AppendPart(
                sb,
                "/_rels/.rels",
                "application/vnd.openxmlformats-package.relationships+xml",
                RootRelsXml());

            AppendPart(
                sb,
                "/word/document.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
                StripXmlDeclaration(documentXml));

            AppendPart(
                sb,
                "/word/_rels/document.xml.rels",
                "application/vnd.openxmlformats-package.relationships+xml",
                StripXmlDeclaration(documentRelsXml));

            AppendPart(
                sb,
                "/word/styles.xml",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml",
                StripXmlDeclaration(templateStylesXml));

            if (!string.IsNullOrEmpty(numberingXml))
            {
                AppendPart(
                    sb,
                    "/word/numbering.xml",
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml",
                    StripXmlDeclaration(numberingXml));
            }

            sb.Append("</pkg:package>");
            return sb.ToString();
        }

        /// <summary>
        /// Liest styles.xml (+ optional numbering.xml) aus DOCX/DOTM/DOTX mit Share-Read.
        /// </summary>
        public static void ReadTemplatePartsFromDocx(
            string docxPath,
            out string stylesXml,
            out string numberingXml)
        {
            stylesXml = null;
            numberingXml = null;
            using (var stream = new FileStream(
                docxPath,
                FileMode.Open,
                FileAccess.Read,
                FileShare.ReadWrite | FileShare.Delete))
            using (var zip = new ZipArchive(stream, ZipArchiveMode.Read))
            {
                stylesXml = ReadEntryText(zip, "word/styles.xml");
                if (stylesXml == null)
                {
                    throw new InvalidOperationException(
                        "Die hinterlegte Vorlage enthält kein word/styles.xml.");
                }

                numberingXml = ReadEntryText(zip, "word/numbering.xml");
            }
        }

        /// <summary>Nur styles.xml (Kompatibilität).</summary>
        public static string ReadStylesXmlFromDocx(string docxPath)
        {
            string styles;
            string numbering;
            ReadTemplatePartsFromDocx(docxPath, out styles, out numbering);
            return styles;
        }

        /// <summary>
        /// Vorlagen-Outline behalten, Markdown-Listen (IDs &gt;= 1000) anhängen.
        /// </summary>
        internal static string MergeNumbering(string templateXml, string generatedXml)
        {
            if (string.IsNullOrWhiteSpace(templateXml))
            {
                return string.IsNullOrWhiteSpace(generatedXml) ? null : generatedXml;
            }

            if (string.IsNullOrWhiteSpace(generatedXml))
            {
                return templateXml;
            }

            string listDefs = ExtractNumberingDefs(generatedXml, ListNumberingIdBase);
            if (string.IsNullOrEmpty(listDefs))
            {
                return templateXml;
            }

            int closing = FindNumberingClose(templateXml);
            if (closing < 0)
            {
                return templateXml;
            }

            var merged = new StringBuilder(templateXml.Length + listDefs.Length + 8);
            merged.Append(templateXml, 0, closing);
            if (merged.Length > 0 && merged[merged.Length - 1] != '\n')
            {
                merged.Append('\n');
            }

            merged.Append(listDefs);
            if (!listDefs.EndsWith("\n", StringComparison.Ordinal))
            {
                merged.Append('\n');
            }

            merged.Append(templateXml, closing, templateXml.Length - closing);
            return merged.ToString();
        }

        private static int FindNumberingClose(string xml)
        {
            int a = xml.LastIndexOf("</w:numbering>", StringComparison.Ordinal);
            int b = xml.LastIndexOf("</numbering>", StringComparison.Ordinal);
            if (a < 0) return b;
            if (b < 0) return a;
            return Math.Min(a, b);
        }

        private static string ExtractNumberingDefs(string xml, int minId)
        {
            var output = new StringBuilder();
            ExtractElementsWithMinId(xml, "abstractNum", "abstractNumId", minId, output);
            ExtractElementsWithMinId(xml, "num", "numId", minId, output);
            return output.ToString();
        }

        private static void ExtractElementsWithMinId(
            string xml,
            string localTag,
            string idAttr,
            int minId,
            StringBuilder output)
        {
            string[] openPatterns = new string[]
            {
                "<w:" + localTag + " ",
                "<" + localTag + " "
            };
            string[] closePatterns = new string[]
            {
                "</w:" + localTag + ">",
                "</" + localTag + ">"
            };

            int searchFrom = 0;
            while (searchFrom < xml.Length)
            {
                int start = -1;
                int openLen = 0;
                for (int i = 0; i < openPatterns.Length; i++)
                {
                    int idx = xml.IndexOf(openPatterns[i], searchFrom, StringComparison.Ordinal);
                    if (idx >= 0 && (start < 0 || idx < start))
                    {
                        start = idx;
                        openLen = openPatterns[i].Length;
                    }
                }

                if (start < 0)
                {
                    break;
                }

                int afterOpen = start + openLen;
                int? id = ParseAttrId(xml.Substring(start), idAttr);
                int closeAt = -1;
                int closeLen = 0;
                for (int i = 0; i < closePatterns.Length; i++)
                {
                    int idx = xml.IndexOf(closePatterns[i], afterOpen, StringComparison.Ordinal);
                    if (idx >= 0 && (closeAt < 0 || idx < closeAt))
                    {
                        closeAt = idx;
                        closeLen = closePatterns[i].Length;
                    }
                }

                if (!id.HasValue || closeAt < 0)
                {
                    searchFrom = afterOpen;
                    continue;
                }

                int end = closeAt + closeLen;
                if (id.Value >= minId)
                {
                    output.Append(xml, start, end - start);
                    output.Append('\n');
                }

                searchFrom = end;
            }
        }

        private static int? ParseAttrId(string fragment, string idAttr)
        {
            string[] prefixes = new string[]
            {
                "w:" + idAttr + "=\"",
                idAttr + "=\""
            };
            for (int i = 0; i < prefixes.Length; i++)
            {
                int idx = fragment.IndexOf(prefixes[i], StringComparison.Ordinal);
                if (idx < 0)
                {
                    continue;
                }

                int valueStart = idx + prefixes[i].Length;
                int valueEnd = fragment.IndexOf('"', valueStart);
                if (valueEnd < 0)
                {
                    continue;
                }

                int value;
                if (int.TryParse(
                    fragment.Substring(valueStart, valueEnd - valueStart),
                    NumberStyles.Integer,
                    CultureInfo.InvariantCulture,
                    out value))
                {
                    return value;
                }
            }

            return null;
        }

        private static string ReadEntryText(ZipArchive zip, string name)
        {
            var entry = zip.GetEntry(name);
            if (entry == null)
            {
                foreach (var candidate in zip.Entries)
                {
                    if (string.Equals(
                        candidate.FullName.Replace('\\', '/'),
                        name,
                        StringComparison.OrdinalIgnoreCase))
                    {
                        entry = candidate;
                        break;
                    }
                }
            }

            if (entry == null)
            {
                return null;
            }

            using (var stream = entry.Open())
            using (var reader = new StreamReader(stream, Encoding.UTF8, true))
            {
                return reader.ReadToEnd();
            }
        }

        private static string RootRelsXml()
        {
            return "<Relationships xmlns=\"" + RelNs + "\">"
                + "<Relationship Id=\"rId1\" Type=\"" + OfficeRelNs + "/officeDocument\" Target=\"word/document.xml\"/>"
                + "</Relationships>";
        }

        private static string BuildDocumentRels(string generatedRelsXml, bool keepNumbering)
        {
            var sb = new StringBuilder();
            sb.Append("<Relationships xmlns=\"").Append(RelNs).Append("\">");
            sb.Append("<Relationship Id=\"rIdStyles\" Type=\"")
                .Append(OfficeRelNs)
                .Append("/styles\" Target=\"styles.xml\"/>");

            if (keepNumbering)
            {
                sb.Append("<Relationship Id=\"rIdNumbering\" Type=\"")
                    .Append(OfficeRelNs)
                    .Append("/numbering\" Target=\"numbering.xml\"/>");
            }

            if (!string.IsNullOrEmpty(generatedRelsXml))
            {
                try
                {
                    var doc = new XmlDocument();
                    doc.LoadXml(generatedRelsXml);
                    var nsmgr = new XmlNamespaceManager(doc.NameTable);
                    nsmgr.AddNamespace("r", RelNs);
                    int hyperId = 1;
                    foreach (XmlNode node in doc.SelectNodes("//r:Relationship", nsmgr))
                    {
                        var type = Attr(node, "Type");
                        var target = Attr(node, "Target");
                        var mode = Attr(node, "TargetMode");
                        if (type.IndexOf("/hyperlink", StringComparison.OrdinalIgnoreCase) < 0)
                        {
                            continue;
                        }

                        sb.Append("<Relationship Id=\"rIdHyper")
                            .Append(hyperId++)
                            .Append("\" Type=\"")
                            .Append(EscapeXml(type))
                            .Append("\" Target=\"")
                            .Append(EscapeXml(target))
                            .Append("\"");
                        if (!string.IsNullOrEmpty(mode))
                        {
                            sb.Append(" TargetMode=\"").Append(EscapeXml(mode)).Append("\"");
                        }

                        sb.Append("/>");
                    }
                }
                catch
                {
                }
            }

            sb.Append("</Relationships>");
            return sb.ToString();
        }

        private static string Attr(XmlNode node, string name)
        {
            if (node.Attributes == null || node.Attributes[name] == null)
            {
                return string.Empty;
            }

            return node.Attributes[name].Value ?? string.Empty;
        }

        private static string EscapeXml(string value)
        {
            if (string.IsNullOrEmpty(value))
            {
                return string.Empty;
            }

            return value
                .Replace("&", "&amp;")
                .Replace("\"", "&quot;")
                .Replace("<", "&lt;")
                .Replace(">", "&gt;");
        }

        private static void AppendPart(StringBuilder sb, string name, string contentType, string xml)
        {
            sb.Append("<pkg:part pkg:name=\"")
                .Append(name)
                .Append("\" pkg:contentType=\"")
                .Append(contentType)
                .Append("\">");
            sb.Append("<pkg:xmlData>");
            sb.Append(xml);
            sb.Append("</pkg:xmlData>");
            sb.Append("</pkg:part>");
        }

        private static string StripXmlDeclaration(string xml)
        {
            if (string.IsNullOrEmpty(xml))
            {
                return xml;
            }

            return Regex.Replace(
                xml.TrimStart(),
                @"^<\?xml\b[^>]*\?>",
                string.Empty,
                RegexOptions.IgnoreCase).TrimStart();
        }
    }
}
