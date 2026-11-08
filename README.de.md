# MDoc

**MDoc** verbindet einen schnellen Markdown-Editor mit Microsoft Word
(Windows). Die Konvertierung läuft über einen gemeinsamen Rust-Kern; Desktop,
CLI und Word-Add-in nutzen dieselbe Logik.

English: [README.md](README.md)

Lizenz: [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

## Status

MDoc ist eine **öffentliche Beta**. Funktionen und Dateiformate können sich
ändern; Round-Trips Markdown ↔ Word sind noch nicht für alle Dokumente
verlustfrei.

Es gibt **keine Zusicherung** auf fehlerfreie oder vollständige Funktion.
Nutzung erfolgt auf eigene Verantwortung — wie in den Lizenzen beschrieben
**„AS IS“**, ohne Gewährleistung.

Feedback und Fehlerberichte sind willkommen (GitHub Issues).

## Architektur

- `mdoc-core`: gemeinsames Dokumentmodell und Konvertierung
- `mdoc-cli`: Headless-Konvertierung und Diagnose (`mdoc-cli.exe`)
- `apps/mdoc-desktop`: Tauri-2/React-Editor mit Live-Vorschau (**MDoc**)
- `apps/mdoc-word-addin`: Word-Ribbon (Einfügen / Kopieren / Mapping / Editor)

Konfigurationsdaten liegen unter `%APPDATA%\MDoc` (einmalige Migration von
`mdoc` / `mdword`, falls vorhanden).

## Vorlagen und Style-Mapping

DOCX-Export nutzt `word/styles.xml` einer `.dotx`/`.dotm`/`.docx`. Ein separates
JSON-`StyleProfile` mappt Markdown-Semantik auf Style-IDs. Vorlage und Mapping
bleiben getrennt.

```powershell
cargo run -p mdoc-cli -- styles company.dotx --pretty
cargo run -p mdoc-cli -- convert input.md output.docx `
  --template company.dotx `
  --style-profile profiles/example-style-profile.json
```

## Word-Ribbon

Standard: Gruppe **MDoc** auf der Registerkarte **Start**. Für eine eigene
Registerkarte im Editor-Menü **Ribbon → Tab** wählen und Word neu starten.

Details: `apps/mdoc-word-addin/README.md`.

## Entwicklung

```powershell
cargo test --workspace
cargo run -p mdoc-cli -- convert input.md output.docx
cd apps/mdoc-desktop
npm install
npm run tauri dev
```

Icons aus `inputs/icon.svg` neu erzeugen:

```powershell
cd apps/mdoc-desktop
npm run icons
```

## Branding

- Produktname: **MDoc**
- Farben: Teal `#009194`, Anthrazit `#3c3c3b`
- `inputs/logo.svg` / `logo_white.svg` — Wortmarke
- `inputs/icon.svg` — App-Icon (aus dem Markenzeichen)
- Schrift **Orbitron** (SIL Open Font License) unter `apps/mdoc-desktop/src/assets/fonts/`
