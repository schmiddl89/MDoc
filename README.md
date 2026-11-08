# MDoc

**MDoc** connects a fast Markdown editor with Microsoft Word (Windows).
Conversion runs through a shared Rust core used by the desktop app, CLI, and
Word add-in.

Also available in [Deutsch](README.de.md).

License: [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

## Download

Windows installer (NSIS, current user): see
[GitHub Releases](https://github.com/schmiddl89/MDoc/releases).

The installer is not code-signed yet — Windows SmartScreen may warn on first
run. Prefer the latest release asset `MDoc_*_x64-setup.exe`.

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

Shared preferences live in `%APPDATA%\MDoc\settings.json` (`uiLanguage`,
`ribbonPlacement`, `splitPercent`) and are used by the editor and the Word
add-in. Legacy `ui-language` / `ribbon-placement` files are migrated once.

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

Build a local NSIS installer from the repo root (requires Rust, Node, and a
Windows host with the .NET Framework 4.x C# compiler):

```powershell
.\rebuild-installer.cmd
```

The setup file is written to `target\release\bundle\nsis\`. Git tags matching
`v*` (for example `v1.0.3`) also build the installer on GitHub Actions.

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
