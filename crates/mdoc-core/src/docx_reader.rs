use std::collections::HashMap;
use std::fs;
use std::path::Path;

use docx_rs::{
    DocumentChild, Hyperlink, HyperlinkData, InsertChild, Paragraph, ParagraphChild, Run, RunChild,
    Table, TableCellContent, TableChild, TableRowChild,
};

use crate::error::{Error, Result};
use crate::model::{Block, Document, Inline, ListItem, TableAlignment};
use crate::style::StyleProfile;

pub fn read(path: impl AsRef<Path>, profile: &StyleProfile) -> Result<Document> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| Error::ReadDocxFile {
        path: path.to_owned(),
        source,
    })?;
    from_bytes(&bytes, profile)
}

pub fn from_bytes(bytes: &[u8], profile: &StyleProfile) -> Result<Document> {
    let parsed = docx_rs::read_docx_with_options(
        bytes,
        docx_rs::ReadDocxOptions::default().with_image_previews(false),
    )
    .map_err(|error| Error::ParseDocx(error.to_string()))?;
    let hyperlinks = parsed
        .hyperlinks
        .iter()
        .map(|(id, target, _)| (id.as_str(), target.as_str()))
        .collect::<HashMap<_, _>>();
    let mut blocks = Vec::new();
    let mut pending_list = Vec::new();

    for child in &parsed.document.children {
        match child {
            DocumentChild::Paragraph(paragraph) => {
                let content = paragraph_inlines(paragraph, &hyperlinks, profile);
                if let Some(list) = list_entry(paragraph, content.clone(), &parsed, profile) {
                    pending_list.push(list);
                } else {
                    flush_lists(&mut blocks, &mut pending_list);
                    blocks.push(paragraph_block(paragraph, content, profile));
                }
            }
            DocumentChild::Table(table) => {
                flush_lists(&mut blocks, &mut pending_list);
                blocks.push(table_block(table, &hyperlinks, profile));
            }
            _ => {}
        }
    }
    flush_lists(&mut blocks, &mut pending_list);

    Ok(Document {
        metadata: Default::default(),
        blocks,
    })
}

#[derive(Debug)]
struct ImportedListItem {
    level: usize,
    numbering_id: usize,
    ordered: bool,
    start: u64,
    item: ListItem,
}

fn list_entry(
    paragraph: &Paragraph,
    mut content: Vec<Inline>,
    parsed: &docx_rs::Docx,
    profile: &StyleProfile,
) -> Option<ImportedListItem> {
    let style = paragraph
        .property
        .style
        .as_ref()
        .map(|style| style.val.as_str());
    let numbering = paragraph
        .property
        .numbering_property
        .as_ref()
        .and_then(|property| {
            let id = property.id.as_ref()?.id;
            if id == 0 {
                return None;
            }
            let level = property.level.as_ref().map_or(0, |level| level.val);
            let (ordered, start) = numbering_kind(parsed, id, level);
            Some((id, level, ordered, start))
        })
        .or_else(|| {
            strip_text_list_marker(&mut content).map(|(level, ordered, start)| {
                let id = if ordered {
                    usize::MAX - 2
                } else {
                    usize::MAX - 3
                };
                (id, level, ordered, start)
            })
        })
        .or_else(|| {
            let unordered = profile.paragraphs.unordered_list.as_deref();
            let ordered = profile.paragraphs.ordered_list.as_deref();
            if style.is_some() && style == unordered {
                Some((usize::MAX, 0, false, 1))
            } else if style.is_some() && style == ordered {
                Some((usize::MAX - 1, 0, true, 1))
            } else {
                None
            }
        })?;

    let checked = strip_task_marker(&mut content);
    Some(ImportedListItem {
        numbering_id: numbering.0,
        level: numbering.1,
        ordered: numbering.2,
        start: numbering.3,
        item: ListItem {
            checked,
            blocks: vec![Block::Paragraph { content }],
        },
    })
}

fn numbering_kind(parsed: &docx_rs::Docx, id: usize, level: usize) -> (bool, u64) {
    let Some(numbering) = parsed
        .numberings
        .numberings
        .iter()
        .find(|numbering| numbering.id == id)
    else {
        return (true, 1);
    };
    let Some(abstract_numbering) = parsed
        .numberings
        .abstract_nums
        .iter()
        .find(|abstract_numbering| abstract_numbering.id == numbering.abstract_num_id)
    else {
        return (true, 1);
    };
    let Some(level) = abstract_numbering
        .levels
        .iter()
        .find(|candidate| candidate.level == level)
    else {
        return (true, 1);
    };

    let ordered = !matches!(level.format.val.as_str(), "bullet" | "none");
    let start = serde_json::to_value(&level.start)
        .ok()
        .and_then(|value| value.as_u64())
        .unwrap_or(1);
    (ordered, start)
}

fn flush_lists(blocks: &mut Vec<Block>, pending: &mut Vec<ImportedListItem>) {
    let mut index = 0;
    while index < pending.len() {
        let level = pending[index].level;
        let (list, next) = build_list(pending, index, level);
        blocks.push(list);
        index = next;
    }
    pending.clear();
}

fn build_list(entries: &[ImportedListItem], mut index: usize, level: usize) -> (Block, usize) {
    let ordered = entries[index].ordered;
    let numbering_id = entries[index].numbering_id;
    let start = entries[index].start;
    let mut items: Vec<ListItem> = Vec::new();

    while index < entries.len() {
        let entry = &entries[index];
        if entry.level < level
            || (entry.level == level
                && (entry.ordered != ordered || entry.numbering_id != numbering_id))
        {
            break;
        }
        if entry.level > level {
            let nested_level = entry.level;
            let (nested, next) = build_list(entries, index, nested_level);
            if let Some(item) = items.last_mut() {
                item.blocks.push(nested);
            }
            index = next;
            continue;
        }
        items.push(entry.item.clone());
        index += 1;
    }

    (
        Block::List {
            ordered,
            start,
            tight: false,
            items,
        },
        index,
    )
}

fn paragraph_block(
    paragraph: &Paragraph,
    mut content: Vec<Inline>,
    profile: &StyleProfile,
) -> Block {
    let plain_text = crate::model::inline_plain_text(&content);
    if let Some((name, body)) = footnote_definition(&plain_text) {
        return Block::FootnoteDefinition {
            name,
            blocks: vec![Block::Paragraph {
                content: vec![Inline::Text { value: body }],
            }],
        };
    }
    let style = paragraph
        .property
        .style
        .as_ref()
        .map(|style| style.val.as_str());
    for level in 1..=6 {
        if style == profile.paragraphs.heading(level) {
            return Block::Heading { level, content };
        }
    }
    if style == profile.paragraphs.block_quote.as_deref() {
        return Block::BlockQuote {
            blocks: vec![Block::Paragraph { content }],
        };
    }
    if style == profile.paragraphs.code_block.as_deref() {
        let language = take_code_language(&mut content);
        let text = crate::model::inline_plain_text(&content);
        if language.is_none() && is_front_matter(&text) {
            return Block::FrontMatter { raw: text };
        }
        return Block::CodeBlock { language, text };
    }
    Block::Paragraph { content }
}

fn footnote_definition(value: &str) -> Option<(String, String)> {
    let value = value.strip_prefix("[^")?;
    let (name, body) = value.split_once("]:")?;
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return None;
    }
    Some((name.to_owned(), body.trim_start().to_owned()))
}

fn is_front_matter(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("---\n") && trimmed.ends_with("\n---")
}

fn take_code_language(content: &mut Vec<Inline>) -> Option<String> {
    let Inline::Strong {
        content: strong_content,
    } = content.first()?
    else {
        return None;
    };
    let [Inline::Code { value }] = strong_content.as_slice() else {
        return None;
    };
    let language = value.strip_suffix('\n')?;
    if language.is_empty()
        || !language.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '#')
        })
    {
        return None;
    }
    let language = language.to_owned();
    content.remove(0);
    Some(language)
}

fn table_block(table: &Table, hyperlinks: &HashMap<&str, &str>, profile: &StyleProfile) -> Block {
    let mut rows = Vec::new();
    for child in &table.rows {
        let TableChild::TableRow(row) = child;
        let mut cells = Vec::new();
        for child in &row.cells {
            let TableRowChild::TableCell(cell) = child;
            let mut content = Vec::new();
            for cell_content in &cell.children {
                if let TableCellContent::Paragraph(paragraph) = cell_content {
                    if !content.is_empty() {
                        content.push(Inline::HardBreak);
                    }
                    content.extend(paragraph_inlines(paragraph, hyperlinks, profile));
                }
            }
            cells.push(content);
        }
        rows.push(cells);
    }

    let mut header = if rows.is_empty() {
        Vec::new()
    } else {
        rows.remove(0)
    };
    for cell in &mut header {
        strip_header_bold(cell);
    }
    Block::Table {
        alignments: vec![TableAlignment::None; header.len()],
        header,
        rows,
    }
}

fn strip_header_bold(content: &mut Vec<Inline>) {
    let mut flattened = Vec::new();
    for inline in content.drain(..) {
        match inline {
            Inline::Strong { content } => flattened.extend(content),
            inline => flattened.push(inline),
        }
    }
    *content = flattened;
}

fn paragraph_inlines(
    paragraph: &Paragraph,
    hyperlinks: &HashMap<&str, &str>,
    profile: &StyleProfile,
) -> Vec<Inline> {
    children_inlines(&paragraph.children, hyperlinks, profile)
}

fn children_inlines(
    children: &[ParagraphChild],
    hyperlinks: &HashMap<&str, &str>,
    profile: &StyleProfile,
) -> Vec<Inline> {
    let mut output = Vec::new();
    for child in children {
        match child {
            ParagraphChild::Run(run) => output.extend(run_inlines(run, profile)),
            ParagraphChild::Hyperlink(link) => {
                output.push(hyperlink_inline(link, hyperlinks, profile));
            }
            ParagraphChild::Insert(insert) => {
                for child in &insert.children {
                    if let InsertChild::Run(run) = child {
                        output.extend(run_inlines(run, profile));
                    }
                }
            }
            _ => {}
        }
    }
    output
}

fn hyperlink_inline(
    hyperlink: &Hyperlink,
    hyperlinks: &HashMap<&str, &str>,
    profile: &StyleProfile,
) -> Inline {
    let (url, title) = match &hyperlink.link {
        HyperlinkData::External { rid, .. } => (
            hyperlinks.get(rid.as_str()).copied().unwrap_or_default(),
            String::new(),
        ),
        HyperlinkData::Anchor { anchor } => ("", anchor.clone()),
    };
    let url = if url.is_empty() && !title.is_empty() {
        format!("#{title}")
    } else {
        url.to_owned()
    };
    Inline::Link {
        url,
        title: String::new(),
        content: children_inlines(&hyperlink.children, hyperlinks, profile),
    }
}

fn run_inlines(run: &Run, profile: &StyleProfile) -> Vec<Inline> {
    let mut content = Vec::new();
    for child in &run.children {
        match child {
            RunChild::Text(text) => content.extend(text_inlines(&text.text)),
            RunChild::Break(_) | RunChild::CarriageReturn(_) => content.push(Inline::HardBreak),
            RunChild::Tab(_) | RunChild::PTab(_) => content.push(Inline::Text {
                value: "\t".to_owned(),
            }),
            _ => {}
        }
    }
    if content.is_empty() {
        return content;
    }

    let properties = serde_json::to_value(&run.run_property).unwrap_or_default();
    let style = run
        .run_property
        .style
        .as_ref()
        .map(|style| style.val.as_str());
    let monospace = style_matches(style, profile.characters.inline_code.as_deref())
        || properties
            .get("fonts")
            .and_then(|fonts| fonts.as_object())
            .is_some_and(|fonts| {
                fonts
                    .values()
                    .filter_map(|value| value.as_str())
                    .any(|font| {
                        font.eq_ignore_ascii_case("Consolas")
                            || font.to_ascii_lowercase().contains("courier")
                    })
            });
    let strong_emphasis =
        style_matches(style, profile.characters.strong_emphasis.as_deref());
    let bold = json_bool(&properties, "bold")
        || strong_emphasis
        || style_matches(style, profile.characters.strong.as_deref());
    let italic = json_bool(&properties, "italic")
        || strong_emphasis
        || style_matches(style, profile.characters.emphasis.as_deref());
    let strike = json_bool(&properties, "strike")
        || json_bool(&properties, "dstrike")
        || style_matches(style, profile.characters.strikethrough.as_deref());

    if monospace {
        content = vec![Inline::Code {
            value: crate::model::inline_plain_text(&content),
        }];
    }
    if strike {
        content = vec![Inline::Strikethrough { content }];
    }
    if italic {
        content = vec![Inline::Emphasis { content }];
    }
    if bold {
        content = vec![Inline::Strong { content }];
    }
    content
}

fn text_inlines(value: &str) -> Vec<Inline> {
    let mut output = Vec::new();
    let mut remainder = value;
    while let Some(start) = remainder.find("[^") {
        let after_start = &remainder[start + 2..];
        let Some(end) = after_start.find(']') else {
            break;
        };
        let name = &after_start[..end];
        if name.is_empty() || name.chars().any(char::is_whitespace) {
            break;
        }
        if start > 0 {
            output.push(Inline::Text {
                value: remainder[..start].to_owned(),
            });
        }
        output.push(Inline::FootnoteReference {
            name: name.to_owned(),
        });
        remainder = &after_start[end + 1..];
    }
    if !remainder.is_empty() {
        output.push(Inline::Text {
            value: remainder.to_owned(),
        });
    }
    output
}

fn json_bool(value: &serde_json::Value, name: &str) -> bool {
    value.get(name).and_then(serde_json::Value::as_bool) == Some(true)
}

fn style_matches(actual: Option<&str>, mapped: Option<&str>) -> bool {
    mapped.is_some_and(|mapped| actual == Some(mapped))
}

fn strip_task_marker(content: &mut [Inline]) -> Option<bool> {
    for inline in content {
        let Inline::Text { value } = inline else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        for (marker, checked) in [
            ("\u{2612} ", true),
            ("\u{2610} ", false),
            ("[x] ", true),
            ("[X] ", true),
            ("[ ] ", false),
        ] {
            if let Some(rest) = value.strip_prefix(marker) {
                *value = rest.to_owned();
                return Some(checked);
            }
        }
        return None;
    }
    None
}

fn strip_text_list_marker(content: &mut [Inline]) -> Option<(usize, bool, u64)> {
    let mut leading_spaces = 0;
    for inline in content {
        let Inline::Text { value } = inline else {
            continue;
        };
        if value.trim().is_empty() {
            leading_spaces += value.chars().filter(|character| *character == ' ').count();
            continue;
        }

        let trimmed = value.trim_start_matches(' ');
        leading_spaces += value.len() - trimmed.len();
        if let Some(rest) = trimmed.strip_prefix("\u{2022} ") {
            *value = rest.to_owned();
            return Some((leading_spaces / 2, false, 1));
        }

        let digit_count = trimmed.chars().take_while(char::is_ascii_digit).count();
        if digit_count > 0 && trimmed[digit_count..].starts_with(". ") {
            let start = trimmed[..digit_count].parse().unwrap_or(1);
            *value = trimmed[digit_count + 2..].to_owned();
            return Some((leading_spaces / 2, true, start));
        }
        return None;
    }
    None
}
