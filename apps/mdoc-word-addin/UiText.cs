namespace MdocWordAddIn
{
    /// <summary>
    /// UI strings for the current language from AppSettings (settings.json).
    /// Ribbon labels require a Word restart after language changes.
    /// </summary>
    internal static class UiText
    {
        internal static string Language
        {
            get { return AppSettings.UiLanguage; }
        }

        internal static bool IsGerman
        {
            get { return Language == "de"; }
        }

        internal static string T(string german, string english)
        {
            return IsGerman ? german : english;
        }

        // --- Shared messages ---

        internal static string ActionFailed
        {
            get
            {
                return T(
                    "MDoc-Aktion fehlgeschlagen:\n",
                    "MDoc action failed:\n");
            }
        }

        internal static string DesktopExeMissing
        {
            get
            {
                return T(
                    "mdoc-desktop.exe wurde nicht gefunden. Bitte MDoc erneut installieren.",
                    "mdoc-desktop.exe was not found. Please reinstall MDoc.");
            }
        }

        internal static string WordUnavailable
        {
            get
            {
                return T(
                    "Word-Anwendung ist nicht verfügbar.",
                    "Word application is not available.");
            }
        }

        internal static string ClipboardEmpty
        {
            get
            {
                return T(
                    "Zwischenablage enthält keinen Text (Markdown erwartet).",
                    "Clipboard does not contain text (Markdown expected).");
            }
        }

        internal static string NoActiveDocument
        {
            get
            {
                return T(
                    "Kein aktives Word-Dokument.",
                    "No active Word document.");
            }
        }

        internal static string DocxConversionEmpty
        {
            get
            {
                return T(
                    "DOCX-Konvertierung lieferte keine Datei.",
                    "DOCX conversion produced no file.");
            }
        }

        internal static string SelectTextFirst
        {
            get
            {
                return T(
                    "Bitte zuerst Text in Word markieren.",
                    "Please select text in Word first.");
            }
        }

        internal static string NoSelectionToInsert
        {
            get
            {
                return T(
                    "Keine Word-Auswahl zum Einfügen.",
                    "No Word selection to insert into.");
            }
        }

        internal static string InsertXmlFailed(string detail)
        {
            return T(
                "Einfügen per InsertXML fehlgeschlagen: " + detail,
                "Insert via InsertXML failed: " + detail);
        }

        internal static string CliMissing
        {
            get
            {
                return T(
                    "mdoc-cli.exe wurde nicht gefunden. Bitte MDoc erneut installieren.",
                    "mdoc-cli.exe was not found. Please reinstall MDoc.");
            }
        }

        internal static string CliStartFailed
        {
            get
            {
                return T(
                    "mdoc-cli konnte nicht gestartet werden.",
                    "Could not start mdoc-cli.");
            }
        }

        internal static string CliTimeout
        {
            get
            {
                return T(
                    "mdoc-cli hat zu lange gebraucht.",
                    "mdoc-cli timed out.");
            }
        }

        internal static string TemplateNotConfigured
        {
            get
            {
                return T(
                    "Keine Word-Vorlage hinterlegt.\n\n"
                    + "Bitte im MDoc-Editor eine .dotx/.dotm wählen und das "
                    + "Formatvorlagen-Mapping speichern.",
                    "No Word template configured.\n\n"
                    + "Please choose a .dotx/.dotm in the MDoc editor and save the style mapping.");
            }
        }

        internal static string TemplateInvalid
        {
            get
            {
                return T(
                    "Hinterlegte Vorlage nicht gefunden oder ungültig.\n\n"
                    + "Bitte im MDoc-Editor die Vorlage erneut wählen und das Mapping speichern.",
                    "Configured template not found or invalid.\n\n"
                    + "Please choose the template again in the MDoc editor and save the mapping.");
            }
        }

        internal static string TemplateLocked(string path)
        {
            return T(
                "Die Word-Vorlage ist gesperrt und styles/numbering konnten weder aus der "
                + "Datei noch über Word gelesen werden.\n\n"
                + "Datei: " + path,
                "The Word template is locked and styles/numbering could not be read from "
                + "the file or via Word.\n\n"
                + "File: " + path);
        }

        internal static string TemplateMissingStylesXml
        {
            get
            {
                return T(
                    "Die hinterlegte Vorlage enthält kein word/styles.xml.",
                    "The configured template has no word/styles.xml.");
            }
        }

        internal static string DocxMissingDocumentXml
        {
            get
            {
                return T(
                    "DOCX enthält kein word/document.xml.",
                    "DOCX has no word/document.xml.");
            }
        }

        internal static string FlatOpcNeedsStyles
        {
            get
            {
                return T(
                    "styles.xml der Vorlage fehlt — Einfügen abgebrochen.",
                    "Template styles.xml is missing — insert aborted.");
            }
        }

        internal static string CliFailedExit(int exitCode, string detail)
        {
            return T(
                "mdoc-cli fehlgeschlagen (Exit " + exitCode + "): " + detail,
                "mdoc-cli failed (exit " + exitCode + "): " + detail);
        }
    }
}
