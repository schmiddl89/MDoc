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

**Einfügen / Start:** mtime+Größe prüfen. Unverändert → Cache. Geändert → Share-Read. Gesperrt → Cache bzw. Word-COM. InsertXML nutzt Vorlagen-`styles` + gemergte `numbering` (Outline aus Vorlage, Listen ab numId 1000 aus der generierten DOCX).

## Ribbon-Aktionen

| Button | Verhalten |
|--------|-----------|
| **Einfügen** | Markdown aus der Zwischenablage → `mdoc-cli convert` → Temp-DOCX → **InsertXML** (Inhalt + `styles.xml` aus der Editor-Vorlage) |
| **Kopieren** | Word-Auswahl → Temp-DOCX → `mdoc-cli import` → Markdown in die Zwischenablage |
| **Formatvorlagen** | startet `mdoc-desktop.exe --map-styles` |
| **Editor** | startet den MDoc-Editor |

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

## Ribbon-Platzierung umschalten

Das Add-in kann an zwei Stellen im Menüband erscheinen:

| Wert | Ergebnis |
|------|----------|
| `home` | Gruppe **MDoc** auf der Registerkarte **Start** (Standard) |
| `ownTab` | eigene Registerkarte **MDoc** |

### So wechselst du die Platzierung

1. Word **vollständig beenden** (alle Fenster schließen).
2. Im Explorer die Adresszeile öffnen und eingeben: `%APPDATA%\MDoc`
3. Dort eine Textdatei namens **`ribbon-placement`** anlegen (ohne `.txt`-Endung).
4. In die Datei **nur eine Zeile** schreiben, z. B. `ownTab` oder `home`.
5. Speichern und Word neu starten.

Fehlt die Datei oder ist sie leer, gilt **`home`**.

### Schnellvariante (PowerShell)

```powershell
New-Item -ItemType Directory -Force "$env:APPDATA\MDoc" | Out-Null
Set-Content -Path "$env:APPDATA\MDoc\ribbon-placement" -Value "ownTab" -Encoding ascii
```
