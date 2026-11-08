# Welcome to MDoc

**MDoc** connects a Markdown editor with Microsoft Word.
This file is the short guide. Use **Help** anytime to open it again in its own window.

## Editor (desktop)

| Action | Meaning |
| :--- | :--- |
| **New** / **Open** / **Save** | Edit Markdown files. MDoc prompts on unsaved changes (including when closing the window). |
| **Template** | Choose a Word template (`.dotx` / `.dotm` / `.docx`) for DOCX export and Word insert. |
| **Mapping** | Map Markdown roles to your template styles (shared with the Word add-in). |
| **Export DOCX** | Write the current Markdown as a Word document. |
| Split view | Editor on the left, live preview on the right. Drag the divider or double-click (50%). |

Supports headings, lists, tables, task lists, quotes, code, and math (`$…$` / `$$…$$`), among other things.

Inline example: $E = mc^2$

## First-time Word setup

1. Choose a **Template** (during install or later via **Template** in the editor).
2. Save the **Mapping**.
3. Restart Word if the ribbon is not visible yet.

Shared files live under `%APPDATA%\MDoc\` (template path, style profile, styles cache).

## Word add-in (ribbon)

The **MDoc** group appears on the **Home** tab by default:

| Button | Function |
| :--- | :--- |
| **Insert** | Paste Markdown from the clipboard into Word (via `mdoc-cli`). |
| **Copy** | Put the Word selection on the clipboard as Markdown. |
| **Styles** | Open the mapping dialog in the editor. |
| **Editor** | Start MDoc Desktop. |

### Ribbon: Home tab or dedicated tab

In the editor menu (☰), use **Ribbon → Home / Tab**:

| Choice | Result |
| :--- | :--- |
| **Home** | **MDoc** group on Word’s **Home** tab (default) |
| **Tab** | Dedicated **MDoc** tab |

Restart Word completely after changing. Preferences are stored in `%APPDATA%\MDoc\settings.json`.

Language and Help are also in the menu (**Language → DE / EN**). Language is shared with the Word add-in via `settings.json` (restart Word after changing).

## Tips

- You can open several editor windows — e.g. this help next to your document.
- Conversion always goes through the shared Rust core (`mdoc-cli`), not duplicated in editor and add-in.
