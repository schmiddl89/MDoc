# MDoc

**MDoc** connects a fast Markdown editor with Microsoft Word (Windows).
Conversion runs through a shared Rust core used by the desktop app, CLI, and
Word add-in.

Also available in [Deutsch](README.de.md).

License: [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

## Status

MDoc is a **public beta**. Features and file formats may change; Markdown ↔ Word
round-trips are not yet lossless for every document.

There is **no guarantee** of correct or complete operation. Use at your own
risk — as stated in the licenses, the software is provided **“AS IS”**, without
warranty.

Feedback and bug reports are welcome via GitHub Issues.

## Architecture

- `mdoc-core`: shared document model and conversion
- `mdoc-cli`: headless conversion and diagnostics (`mdoc-cli.exe`)
- `apps/mdoc-desktop`: Tauri 2 / React editor with live preview (**MDoc**)
- `apps/mdoc-word-addin`: Word ribbon (Insert / Copy / Mapping / Editor)

Config files live under `%APPDATA%\MDoc` (one-time migration from `mdoc` /
`mdword` if present).

UI language is stored in `%APPDATA%\MDoc\ui-language` (`de` or `en`) and is
shared by the editor and the Word add-in.

## Templates and style mapping

DOCX export uses `word/styles.xml` from a `.dotx` / `.dotm` / `.docx`. A separate
JSON `StyleProfile` maps Markdown semantics to style IDs. Template and mapping
stay separate.

```powershell
cargo run -p mdoc-cli -- styles company.dotx --pretty
cargo run -p mdoc-cli -- convert input.md output.docx `
  --template company.dotx `
  --style-profile profiles/example-style-profile.json
```

## Word ribbon

Default: **MDoc** group on the **Home** tab. Switch to a dedicated tab in the
editor menu (**Ribbon → Tab**), then restart Word.

Details: `apps/mdoc-word-addin/README.md`.

## Development

```powershell
cargo test --workspace
cargo run -p mdoc-cli -- convert input.md output.docx
cd apps/mdoc-desktop
npm install
npm run tauri dev
```

Regenerate icons from `inputs/icon.svg`:

```powershell
cd apps/mdoc-desktop
npm run icons
```

## Branding

- Product name: **MDoc**
- Colors: teal `#009194`, charcoal `#3c3c3b`
- `inputs/logo.svg` / `logo_white.svg` — wordmark
- `inputs/icon.svg` — app icon (from the mark)
- Font **Orbitron** (SIL Open Font License) under `apps/mdoc-desktop/src/assets/fonts/`
