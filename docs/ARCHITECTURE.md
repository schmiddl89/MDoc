# MDoc architecture

- The Rust crate `mdoc-core` is the shared source of all conversion logic.
- CLI, desktop app, and Word adapter must not duplicate Markdown/DOCX
  conversion logic.
- Interoperability is developed with public standards, self-made fixtures, and
  golden-file tests.
- License, activation, or telemetry mechanisms from other products are not
  copied or bypassed.
- The default install should work per user. Deep Word text-converter
  integration may be added later as an optional elevated install.
- New DOCX or Markdown features need at least one round-trip or structure test.
- The React preview may render Markdown directly. File conversion and semantic
  analysis must always use the shared Rust core.
- The base installer is a Tauri NSIS package in `currentUser` mode.
  Admin-required Word system integration remains a separate, explicitly chosen
  install module.
- Markdown semantics are mapped to OOXML/Word style IDs via `StyleProfile`.
  Style names must not be hard-wired in parser or adapter logic.
- DOCX import and the Word reader use the same `StyleProfile` inversely so
  export, import, and later COM integration share semantics.
- Custom `.dotx`/`.docx` templates supply formatting styles on export; the
  Markdown mapping stays in a separate, serializable profile.
- The Word adapter should insert text in blocks and then format ranges.
  Character-by-character COM calls should be avoided.
