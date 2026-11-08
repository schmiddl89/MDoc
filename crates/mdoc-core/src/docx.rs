use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use docx_rs::{
    AbstractNumbering, BreakType, Docx, Hyperlink, HyperlinkType, IndentLevel, Level, LevelJc,
    LevelOverride, LevelText, NumberFormat, Numbering, NumberingId, Paragraph, Run, RunFonts,
    SpecialIndentType, Start, Style, StyleType, Table, TableCell, TableRow,
};

use crate::error::{Error, Result};
use crate::model::{Block, Document, Inline, ListItem, TableCell as ModelTableCell};
use crate::style::{StyleProfile, style_ui_name};

/// High IDs so Markdown lists do not collide with template outline numbering
/// (corporate templates often use numId 1–9 for headings).
pub(crate) const LIST_NUMBERING_ID_BASE: usize = 1000;
const BULLET_ABSTRACT_ID: usize = LIST_NUMBERING_ID_BASE;
const BULLET_NUM_ID: usize = LIST_NUMBERING_ID_BASE;
const ORDERED_ABSTRACT_ID: usize = LIST_NUMBERING_ID_BASE + 1;
const ORDERED_NUM_BASE: usize = LIST_NUMBERING_ID_BASE + 1;
const MAX_LIST_LEVEL: usize = 8;

#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    pub style_profile: StyleProfile,
    pub template: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default)]
struct InlineStyle<'a> {
    bold: bool,
    italic: bool,
    strike: bool,
    monospace: bool,
    character_style: Option<&'a str>,
}

struct WriterState<'a> {
    profile: &'a StyleProfile,
    next_ordered_num_id: usize,
}

pub fn write(document: &Document, path: impl AsRef<Path>) -> Result<()> {
    write_with_options(document, path, &ExportOptions::default())
}

pub fn write_with_profile(
    document: &Document,
    path: impl AsRef<Path>,
    profile: &StyleProfile,
) -> Result<()> {
    write_with_options(
        document,
        path,
        &ExportOptions {
            style_profile: profile.clone(),
            template: None,
        },
    )
}

pub fn write_with_options(
    document: &Document,
    path: impl AsRef<Path>,
    options: &ExportOptions,
) -> Result<()> {
    let path = path.as_ref();
    let file = File::create(path).map_err(|source| Error::CreateDocx {
        path: path.to_owned(),
        source,
    })?;

    if let Some(template) = &options.template {
        let mut generated = Cursor::new(Vec::new());
        build_with_profile(document, &options.style_profile)
            .build()
            .pack(&mut generated)
            .map_err(|error| Error::WriteDocx(error.to_string()))?;
        crate::template::apply_styles(&generated.into_inner(), template, file)
    } else {
        build_with_profile(document, &options.style_profile)
            .build()
            .pack(file)
            .map_err(|error| Error::WriteDocx(error.to_string()))
    }
}

#[must_use]
pub fn build(document: &Document) -> Docx {
    build_with_profile(document, &StyleProfile::default())
}

#[must_use]
pub fn build_with_profile(document: &Document, profile: &StyleProfile) -> Docx {
    let mut state = WriterState {
        profile,
        next_ordered_num_id: ORDERED_NUM_BASE,
    };
    let docx = with_profile_styles(
        Docx::new()
            .add_abstract_numbering(bullet_abstract())
            .add_numbering(Numbering::new(BULLET_NUM_ID, BULLET_ABSTRACT_ID))
            .add_abstract_numbering(ordered_abstract()),
        profile,
    );
    add_blocks(docx, &document.blocks, &mut state)
}

/// Nur Überschriften stubben (Built-in-Name `heading N` → Galerie).
/// Keine Zeichen-/Listen-/Code-Stubs: FormattedText würde sie sonst als neue Kacheln
/// importieren (z. B. „Fett Kursiv“ neben bestehendem „Fett & Kursiv“), inkl. Umsortierung.
/// Absätze/Runs referenzieren weiter die Mapping-IDs; das Add-in mappt auf NameLocal im Ziel.
fn with_profile_styles(mut docx: Docx, profile: &StyleProfile) -> Docx {
    let base_id = profile
        .paragraphs
        .normal
        .as_deref()
        .unwrap_or("Normal");
    // Basis ohne qFormat — sonst kann Insert die Galerie-Reihenfolge anfassen.
    docx = docx.add_style(
        Style::new(base_id, StyleType::Paragraph).name(style_ui_name(base_id)),
    );

    let headings = [
        (1u8, profile.paragraphs.heading1.as_deref()),
        (2, profile.paragraphs.heading2.as_deref()),
        (3, profile.paragraphs.heading3.as_deref()),
        (4, profile.paragraphs.heading4.as_deref()),
        (5, profile.paragraphs.heading5.as_deref()),
        (6, profile.paragraphs.heading6.as_deref()),
    ];
    for (level, style_id) in headings {
        if let Some(style_id) = style_id {
            docx = docx.add_style(
                Style::new(style_id, StyleType::Paragraph)
                    .name(format!("heading {level}"))
                    .based_on(base_id)
                    .outline_lvl(usize::from(level.saturating_sub(1)))
                    .q_format(true),
            );
        }
    }

    docx
}

fn bullet_abstract() -> AbstractNumbering {
    let mut numbering = AbstractNumbering::new(BULLET_ABSTRACT_ID);
    let markers = ["\u{2022}", "o", "\u{25AA}"];
    for level in 0..=MAX_LIST_LEVEL {
        let left = 720 * (level as i32 + 1);
        numbering = numbering.add_level(
            Level::new(
                level,
                Start::new(1),
                NumberFormat::new("bullet"),
                LevelText::new(markers[level % markers.len()]),
                LevelJc::new("left"),
            )
            .indent(
                Some(left),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );
    }
    numbering
}

fn ordered_abstract() -> AbstractNumbering {
    let mut numbering = AbstractNumbering::new(ORDERED_ABSTRACT_ID);
    let formats = ["%1.", "%2.", "%3.", "%4.", "%5.", "%6.", "%7.", "%8.", "%9."];
    for level in 0..=MAX_LIST_LEVEL {
        let left = 720 * (level as i32 + 1);
        numbering = numbering.add_level(
            Level::new(
                level,
                Start::new(1),
                NumberFormat::new("decimal"),
                LevelText::new(formats[level]),
                LevelJc::new("left"),
            )
            .indent(
                Some(left),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );
    }
    numbering
}

fn add_blocks(mut docx: Docx, blocks: &[Block], state: &mut WriterState<'_>) -> Docx {
    for block in blocks {
        docx = match block {
            Block::Paragraph { content } => docx.add_paragraph(paragraph_from_inlines(
                content,
                state.profile.paragraphs.normal.as_deref(),
                state.profile,
            )),
            Block::Heading { level, content } => {
                // Do not set outlineLvl on the paragraph: template heading styles
                // already define outline + multilevel numbering. Direct outlineLvl
                // would fight the style and remapped numbering.
                docx.add_paragraph(paragraph_from_inlines(
                    content,
                    state.profile.paragraphs.heading(*level),
                    state.profile,
                ))
            }
            Block::BlockQuote { blocks } => {
                let paragraph = paragraph_with_optional_style(
                    Paragraph::new(),
                    state.profile.paragraphs.block_quote.as_deref(),
                );
                if let [Block::Paragraph { content }] = blocks.as_slice() {
                    docx.add_paragraph(append_inlines(
                        paragraph,
                        content,
                        InlineStyle::default(),
                        state.profile,
                    ))
                } else {
                    docx.add_paragraph(
                        paragraph.add_run(Run::new().add_text(blocks_plain_text(blocks))),
                    )
                }
            }
            Block::CodeBlock { language, text } => {
                let mut paragraph = paragraph_with_optional_style(
                    Paragraph::new(),
                    state.profile.paragraphs.code_block.as_deref(),
                );
                if let Some(language) = language {
                    paragraph = paragraph.add_run(
                        Run::new()
                            .bold()
                            .fonts(monospace_fonts())
                            .add_text(language)
                            .add_break(BreakType::TextWrapping),
                    );
                }
                paragraph = add_multiline_run(
                    paragraph,
                    text,
                    InlineStyle {
                        monospace: true,
                        character_style: state.profile.characters.inline_code.as_deref(),
                        ..InlineStyle::default()
                    },
                );
                docx.add_paragraph(paragraph)
            }
            Block::List {
                ordered,
                start,
                items,
                ..
            } => {
                let (docx_with_num, num_id) = ensure_list_numbering(docx, *ordered, *start, state);
                add_list(docx_with_num, *ordered, items, 0, num_id, state)
            }
            Block::Table { header, rows, .. } => {
                let mut table_rows =
                    Vec::with_capacity(rows.len() + usize::from(!header.is_empty()));
                if !header.is_empty() {
                    table_rows.push(table_row(header, true, state.profile));
                }
                table_rows.extend(
                    rows.iter()
                        .map(|row| table_row(row, false, state.profile)),
                );
                let table = Table::new(table_rows);
                let table = match state.profile.paragraphs.table.as_deref() {
                    Some(style) => table.style(style),
                    None => table,
                };
                docx.add_table(table)
            }
            Block::ThematicBreak => docx.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text("\u{2500}".repeat(24))),
            ),
            Block::FrontMatter { raw } => {
                let paragraph = add_multiline_run(
                    paragraph_with_optional_style(
                        Paragraph::new(),
                        state.profile.paragraphs.code_block.as_deref(),
                    ),
                    raw,
                    InlineStyle {
                        monospace: true,
                        character_style: state.profile.characters.inline_code.as_deref(),
                        ..InlineStyle::default()
                    },
                );
                docx.add_paragraph(paragraph)
            }
            Block::Html { raw } => {
                let paragraph = paragraph_with_optional_style(
                    Paragraph::new(),
                    state.profile.paragraphs.code_block.as_deref(),
                );
                docx.add_paragraph(
                    paragraph.add_run(
                        styled_run(InlineStyle {
                            monospace: true,
                            character_style: state.profile.characters.inline_code.as_deref(),
                            ..InlineStyle::default()
                        })
                        .add_text(raw),
                    ),
                )
            }
            Block::FootnoteDefinition { name, blocks } => {
                let text = format!("[^{name}]: {}", blocks_plain_text(blocks));
                docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)))
            }
        };
    }
    docx
}

fn ensure_list_numbering(
    docx: Docx,
    ordered: bool,
    start: u64,
    state: &mut WriterState<'_>,
) -> (Docx, usize) {
    if !ordered {
        return (docx, BULLET_NUM_ID);
    }

    let num_id = state.next_ordered_num_id;
    state.next_ordered_num_id += 1;
    let start = usize::try_from(start.max(1)).unwrap_or(1);
    let numbering = Numbering::new(num_id, ORDERED_ABSTRACT_ID)
        .add_override(LevelOverride::new(0).start(start));
    (docx.add_numbering(numbering), num_id)
}

fn add_list(
    mut docx: Docx,
    ordered: bool,
    items: &[ListItem],
    depth: usize,
    num_id: usize,
    state: &mut WriterState<'_>,
) -> Docx {
    let style = if ordered {
        state.profile.paragraphs.ordered_list.as_deref()
    } else {
        state.profile.paragraphs.unordered_list.as_deref()
    };
    let level = depth.min(MAX_LIST_LEVEL);

    for item in items {
        docx = add_list_item_blocks(docx, &item.blocks, ordered, level, num_id, item.checked, style, state);
    }
    docx
}

fn add_list_item_blocks(
    mut docx: Docx,
    blocks: &[Block],
    ordered: bool,
    depth: usize,
    num_id: usize,
    checked: Option<bool>,
    list_style: Option<&str>,
    state: &mut WriterState<'_>,
) -> Docx {
    let mut wrote_primary = false;

    for block in blocks {
        match block {
            Block::Paragraph { content } => {
                let mut paragraph = paragraph_with_optional_style(Paragraph::new(), list_style)
                    .numbering(NumberingId::new(num_id), IndentLevel::new(depth));
                if !wrote_primary {
                    if let Some(marker) = checkbox_prefix(checked) {
                        paragraph = paragraph.add_run(Run::new().add_text(marker));
                    }
                    wrote_primary = true;
                }
                docx = docx.add_paragraph(append_inlines(
                    paragraph,
                    content,
                    InlineStyle::default(),
                    state.profile,
                ));
            }
            Block::List {
                ordered: nested_ordered,
                start,
                items,
                ..
            } => {
                let (docx_with_num, nested_id) = if *nested_ordered == ordered {
                    (docx, num_id)
                } else {
                    ensure_list_numbering(docx, *nested_ordered, *start, state)
                };
                docx = add_list(
                    docx_with_num,
                    *nested_ordered,
                    items,
                    depth + 1,
                    nested_id,
                    state,
                );
                wrote_primary = true;
            }
            other => {
                if !wrote_primary {
                    let mut paragraph = paragraph_with_optional_style(Paragraph::new(), list_style)
                        .numbering(NumberingId::new(num_id), IndentLevel::new(depth));
                    if let Some(marker) = checkbox_prefix(checked) {
                        paragraph = paragraph.add_run(Run::new().add_text(marker));
                    }
                    paragraph = paragraph.add_run(Run::new().add_text(blocks_plain_text(&[other.clone()])));
                    docx = docx.add_paragraph(paragraph);
                    wrote_primary = true;
                } else {
                    docx = add_blocks(docx, std::slice::from_ref(other), state);
                }
            }
        }
    }

    if !wrote_primary {
        let mut paragraph = paragraph_with_optional_style(Paragraph::new(), list_style)
            .numbering(NumberingId::new(num_id), IndentLevel::new(depth));
        if let Some(marker) = checkbox_prefix(checked) {
            paragraph = paragraph.add_run(Run::new().add_text(marker));
        }
        docx = docx.add_paragraph(paragraph);
    }

    docx
}

fn checkbox_prefix(checked: Option<bool>) -> Option<&'static str> {
    match checked {
        Some(true) => Some("\u{2612} "),
        Some(false) => Some("\u{2610} "),
        None => None,
    }
}

fn table_row(cells: &[ModelTableCell], header: bool, profile: &StyleProfile) -> TableRow {
    let cells = cells
        .iter()
        .map(|content| {
            let style = InlineStyle {
                bold: header,
                ..InlineStyle::default()
            };
            TableCell::new().add_paragraph(paragraph_with_style(content, style, profile))
        })
        .collect();
    TableRow::new(cells)
}

fn paragraph_from_inlines(
    inlines: &[Inline],
    style: Option<&str>,
    profile: &StyleProfile,
) -> Paragraph {
    let paragraph = paragraph_with_optional_style(Paragraph::new(), style);
    append_inlines(paragraph, inlines, InlineStyle::default(), profile)
}

fn paragraph_with_style(
    inlines: &[Inline],
    style: InlineStyle<'_>,
    profile: &StyleProfile,
) -> Paragraph {
    append_inlines(Paragraph::new(), inlines, style, profile)
}

fn paragraph_with_optional_style(paragraph: Paragraph, style: Option<&str>) -> Paragraph {
    match style {
        Some(style) => paragraph.style(style),
        None => paragraph,
    }
}

fn append_inlines<'a>(
    mut paragraph: Paragraph,
    inlines: &[Inline],
    style: InlineStyle<'a>,
    profile: &'a StyleProfile,
) -> Paragraph {
    for inline in inlines {
        paragraph = match inline {
            Inline::Text { value } => paragraph.add_run(styled_run(style).add_text(value)),
            Inline::SoftBreak | Inline::HardBreak => {
                paragraph.add_run(styled_run(style).add_break(BreakType::TextWrapping))
            }
            Inline::Emphasis { content } => {
                let character_style = if style.bold {
                    profile
                        .characters
                        .strong_emphasis
                        .as_deref()
                        .or(profile.characters.emphasis.as_deref())
                } else {
                    profile.characters.emphasis.as_deref()
                };
                append_inlines(
                    paragraph,
                    content,
                    InlineStyle {
                        italic: true,
                        character_style,
                        ..style
                    },
                    profile,
                )
            }
            Inline::Strong { content } => {
                let character_style = if style.italic {
                    profile
                        .characters
                        .strong_emphasis
                        .as_deref()
                        .or(profile.characters.strong.as_deref())
                } else {
                    profile.characters.strong.as_deref()
                };
                append_inlines(
                    paragraph,
                    content,
                    InlineStyle {
                        bold: true,
                        character_style,
                        ..style
                    },
                    profile,
                )
            }
            Inline::Strikethrough { content } => append_inlines(
                paragraph,
                content,
                InlineStyle {
                    strike: true,
                    character_style: profile.characters.strikethrough.as_deref(),
                    ..style
                },
                profile,
            ),
            Inline::Code { value } => paragraph.add_run(
                styled_run(InlineStyle {
                    monospace: true,
                    character_style: profile.characters.inline_code.as_deref(),
                    ..style
                })
                .add_text(value),
            ),
            Inline::Math { display, value } => {
                let value = if *display {
                    format!("$${value}$$")
                } else {
                    format!("${value}$")
                };
                paragraph.add_run(
                    styled_run(style)
                        .fonts(
                            RunFonts::new()
                                .ascii("Cambria Math")
                                .hi_ansi("Cambria Math"),
                        )
                        .add_text(value),
                )
            }
            Inline::Link {
                url,
                title: _,
                content,
            } => {
                let text = crate::model::inline_plain_text(content);
                let run = styled_run(style)
                    .style(profile.characters.link.as_deref().unwrap_or("Hyperlink"))
                    .color("0563C1")
                    .underline("single")
                    .add_text(text);
                paragraph.add_hyperlink(
                    Hyperlink::new(url.clone(), HyperlinkType::External).add_run(run),
                )
            }
            Inline::Image { url, alt, .. } => {
                let alt = crate::model::inline_plain_text(alt);
                paragraph.add_run(styled_run(style).add_text(format!("[Image: {alt}] ({url})")))
            }
            Inline::Html { raw } => paragraph.add_run(
                styled_run(InlineStyle {
                    monospace: true,
                    ..style
                })
                .add_text(raw),
            ),
            Inline::FootnoteReference { name } => {
                paragraph.add_run(styled_run(style).add_text(format!("[^{name}]")))
            }
        };
    }
    paragraph
}

fn add_multiline_run(mut paragraph: Paragraph, text: &str, style: InlineStyle<'_>) -> Paragraph {
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            paragraph = paragraph.add_run(styled_run(style).add_break(BreakType::TextWrapping));
        }
        if !line.is_empty() {
            paragraph = paragraph.add_run(styled_run(style).add_text(line));
        }
    }
    paragraph
}

fn styled_run(style: InlineStyle<'_>) -> Run {
    let mut run = Run::new();
    if let Some(character_style) = style.character_style {
        // Nur rStyle setzen — kein zusätzliches bold/italic/strike.
        // Sonst zeigt Word die Vorlage an, wendet sie aber nicht an (Direktformatierung gewinnt),
        // oder legt Built-in-Alternativen an.
        run = run.style(character_style);
        if style.monospace {
            run = run.fonts(monospace_fonts());
        }
        return run;
    }
    if style.bold {
        run = run.bold();
    }
    if style.italic {
        run = run.italic();
    }
    if style.strike {
        run = run.strike();
    }
    if style.monospace {
        run = run.fonts(monospace_fonts());
    }
    run
}

fn monospace_fonts() -> RunFonts {
    RunFonts::new().ascii("Consolas").hi_ansi("Consolas")
}

fn blocks_plain_text(blocks: &[Block]) -> String {
    let mut output = String::new();
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        match block {
            Block::Paragraph { content } | Block::Heading { content, .. } => {
                output.push_str(&crate::model::inline_plain_text(content));
            }
            Block::BlockQuote { blocks } | Block::FootnoteDefinition { blocks, .. } => {
                output.push_str(&blocks_plain_text(blocks));
            }
            Block::CodeBlock { text, .. }
            | Block::FrontMatter { raw: text }
            | Block::Html { raw: text } => output.push_str(text),
            Block::List { items, .. } => {
                for item in items {
                    output.push_str(&blocks_plain_text(&item.blocks));
                    output.push('\n');
                }
            }
            Block::Table { header, rows, .. } => {
                for cell in header.iter().chain(rows.iter().flatten()) {
                    output.push_str(&crate::model::inline_plain_text(cell));
                    output.push('\t');
                }
            }
            Block::ThematicBreak => output.push_str("---"),
        }
    }
    output
}
