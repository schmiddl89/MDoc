# Willkommen bei MDoc

**MDoc** verbindet einen Markdown-Editor mit Microsoft Word.
Diese Datei ist die Kurzanleitung. Über den Button **Hilfe** kannst du sie
jederzeit erneut in einem eigenen Fenster öffnen.

## Editor (Desktop)

| Aktion | Bedeutung |
| :--- | :--- |
| **Neu** / **Öffnen** / **Speichern** | Markdown-Dateien bearbeiten. Bei ungespeicherten Änderungen fragt MDoc nach (auch beim Schließen des Fensters). |
| **Vorlage** | Word-Vorlage (`.dotx` / `.dotm` / `.docx`) für DOCX-Export und Word-Einfügen wählen. |
| **Mapping** | Markdown-Rollen den Formatvorlagen deiner Vorlage zuordnen (gemeinsam mit dem Word-Add-in). |
| **DOCX exportieren** | Aktuelles Markdown als Word-Dokument speichern. |
| Split-Ansicht | Links der Editor, rechts die Live-Vorschau. Den Teiler ziehen oder doppelklicken (50 %). |

Unterstützt unter anderem Überschriften, Listen, Tabellen, Aufgabenlisten, Zitate, Code und Formeln (`$…$` / `$$…$$`).

Inline-Beispiel: $E = mc^2$

## Erste Einrichtung für Word

1. Eine **Vorlage** wählen (beim Installieren oder später über **Vorlage** im Editor).
2. Das **Mapping** speichern.
3. Word neu starten, falls das Ribbon noch nicht sichtbar ist.

Gemeinsame Dateien liegen unter `%APPDATA%\MDoc\` (Vorlagenpfad, Style-Profil, Styles-Cache).

## Word-Add-in (Ribbon)

Die Gruppe **MDoc** erscheint standardmäßig auf der Registerkarte **Start**:

| Button | Funktion |
| :--- | :--- |
| **Einfügen** | Markdown aus der Zwischenablage nach Word übernehmen (über `mdoc-cli`). |
| **Kopieren** | Markierte Word-Auswahl als Markdown in die Zwischenablage legen. |
| **Formatvorlagen** | Mapping-Dialog im Editor öffnen. |
| **Editor** | MDoc-Desktop starten. |

### Ribbon: Start oder eigene Registerkarte

Im Editor-Menü (☰) unter **Ribbon → Start / Tab**:

| Wahl | Ergebnis |
| :--- | :--- |
| **Start** | Gruppe **MDoc** auf Words Registerkarte **Start** (Standard) |
| **Tab** | eigene Registerkarte **MDoc** |

Danach Word vollständig neu starten. Gespeichert unter `%APPDATA%\MDoc\ribbon-placement`.

Sprache und Hilfe liegen ebenfalls im Menü (**Sprache → DE / EN**). Die Sprache wird unter `%APPDATA%\MDoc\ui-language` geteilt; nach Wechsel Word neu starten.

## Tipps

- Du kannst mehrere Editor-Fenster parallel öffnen — z. B. diese Hilfe neben deinem Dokument.
- Konvertierung und Semantik laufen immer über den gemeinsamen Rust-Kern (`mdoc-cli`), nicht doppelt in Editor und Add-in.
