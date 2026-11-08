# MDoc Word-Add-in

Clean-Room COM-Add-in (IDTExtensibility2 + Ribbon) für Microsoft Word.

- Eigene Implementierung, keine Übernahme aus dem PoC
- Nutzt Microsoft Extensibility-/Office-PIAs
- Konvertierung läuft über **`mdoc-cli.exe`** (gemeinsamer Rust-Kern) — keine duplizierte Markdown-/DOCX-Logik
- Fehler werden geloggt unter `%APPDATA%\MDoc\word-addin.log`

## Voraussetzung für Einfügen

1. Word-Vorlage: im Installer wählbar (Standard: `Normal.dotm`, falls vorhanden;
   optional überspringen / später im **MDoc-Editor**).
2. Formatvorlagen-**Mapping** speichern.

Damit entstehen unter `%APPDATA%\MDoc\`:

| Datei | Inhalt |
|--------|--------|
| `template-path.txt` | Absoluter Pfad zur gewählten Vorlage |
| `template-styles.xml` | Cache von `word/styles.xml` (Fallback bei Word-Dateisperre) |
| `template-numbering.xml` | Cache von `word/numbering.xml` (Outline der Überschriften) |
| `template-styles.meta` | Fingerprint: Pfad, mtime (Unix-ms), Größe, numbering |
| `style-profile.json` | Mapping Markdown → Style-IDs |
| `settings.json` | Shared preferences (`uiLanguage`, `ribbonPlacement`, …) |

**Einfügen / Start:** mtime+Größe prüfen. Unverändert → Cache. Geändert → Share-Read. Gesperrt → Cache bzw. Word-COM. InsertXML nutzt Vorlagen-`styles` + gemergte `numbering` (Outline aus Vorlage, Listen ab numId 1000 aus der generierten DOCX).

## Ribbon actions / Ribbon-Aktionen

| Button (EN / DE) | Behavior |
|--------|-----------|
| **Insert** / **Einfügen** | Markdown from clipboard → `mdoc-cli convert` → temp DOCX → **InsertXML** |
| **Copy** / **Kopieren** | Word selection → temp DOCX → `mdoc-cli import` → Markdown on clipboard |
| **Styles** / **Formatvorlagen** | launches `mdoc-desktop.exe --map-styles` |
| **Editor** | launches the MDoc editor |

### UI language and ribbon placement

Ribbon labels, message boxes, and ribbon placement follow `%APPDATA%\MDoc\settings.json`
(`uiLanguage`: `de`/`en`, `ribbonPlacement`: `home`/`ownTab`). Set both in the MDoc
desktop menu; **restart Word** afterwards. Legacy `ui-language` / `ribbon-placement`
files are still read if `settings.json` is missing.

## Build

```powershell
.\build.ps1
```

## Registrierung (benutzerbezogen)

```powershell
.\Register-MdocAddIn.ps1
```

Word **vollständig beenden** und neu starten. `mdoc-cli.exe` muss neben `mdoc-desktop.exe` liegen (Installer legt beides unter `%LOCALAPPDATA%\MDoc` ab).

```powershell
.\Register-MdocAddIn.ps1 -Unregister
```

## Ribbon placement

Choose **Ribbon → Home / Tab** in the MDoc desktop menu, then restart Word.
The value is stored in `settings.json` as `ribbonPlacement` (`home` or `ownTab`).
