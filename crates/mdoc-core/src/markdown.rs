use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment as ComrakTableAlignment};
use comrak::{Arena, Options, parse_document};

use crate::error::{Error, Result};
use crate::model::{Block, Document, Inline, ListItem, TableAlignment, TableCell};

#[must_use]
pub fn default_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.extension.math_dollars = true;
    options.extension.math_latex = true;
    options
}

pub fn parse(markdown: &str) -> Result<Document> {
    parse_with_options(markdown, &default_options())
}

pub fn parse_with_options(markdown: &str, options: &Options<'_>) -> Result<Document> {
    let normalized = normalize_list_indentation(markdown);
    let arena = Arena::new();
    let root = parse_document(&arena, &normalized, options);
    Ok(Document {
        metadata: Default::default(),
        blocks: parse_blocks(root)?,
    })
}

/// Editor-freundliche Listen: je **2 Leerzeichen** eine Ebene (wie bei Aufzählungen),
/// auch unter Nummerierungen. CommonMark verlangt sonst oft 3+ Leerzeichen wegen `1. `,
/// und eine verschachtelte Nummerierung mit `2.` ohne Leerzeile wird sonst als Text gelesen.
///
/// Strategie: Einrückung = 3 Leerzeichen × Ebene; verschachtelte Nummerierungen werden
/// innerhalb der Ebene neu mit 1., 2., … nummeriert (Word/COM nutzt danach eigene Num-IDs).
#[must_use]
pub fn normalize_list_indentation(markdown: &str) -> String {
    let mut output = String::with_capacity(markdown.len().saturating_add(64));
    let mut level_counters: Vec<u64> = Vec::new();
    let mut last_ordered: Option<bool> = None;

    for line in markdown.split_inclusive('\n') {
        let newline = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let content = line
            .strip_suffix("\r\n")
            .or_else(|| line.strip_suffix('\n'))
            .unwrap_or(line);

        if let Some((indent_cols, ordered, marker_suffix)) = split_indented_list_marker(content) {
            // Aufzählung → Nummerierung (oder umgekehrt): Zähler neu starten,
            // sonst wird „1.“ nach zwei Bullets zu „3.“.
            if last_ordered.is_some_and(|previous| previous != ordered) {
                level_counters.clear();
            }
            last_ordered = Some(ordered);

            let level = indent_cols / 2;
            level_counters.truncate(level + 1);
            while level_counters.len() <= level {
                level_counters.push(0);
            }
            level_counters[level] = level_counters[level].saturating_add(1);

            for _ in 0..level {
                // 3 Leerzeichen reichen für Marker der Form `1. ` / `10. ` bis zweistellig
                // auf der ersten Verschachtelung; höhere Ebenen brauchen ebenfalls Vielfache von 3.
                output.push_str("   ");
            }
            if ordered {
                output.push_str(&format!("{}. ", level_counters[level]));
                output.push_str(marker_suffix);
            } else {
                output.push_str(marker_suffix);
            }
        } else {
            if !content.trim().is_empty() {
                // Nicht-Listenzeile: Zähler zurücksetzen (neue Liste danach startet bei 1).
                level_counters.clear();
                last_ordered = None;
            }
            output.push_str(content);
        }
        output.push_str(newline);
    }
    output
}

/// `(indent_columns, is_ordered, rest_after_marker)` — bei Unordered ist `rest` inkl. `- `/`* `/`+ `.
fn split_indented_list_marker(line: &str) -> Option<(usize, bool, &str)> {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut columns = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b' ' => {
                columns += 1;
                index += 1;
            }
            b'\t' => {
                columns += 2;
                index += 1;
            }
            _ => break,
        }
    }
    let rest = &line[index..];
    if rest.starts_with("- ") || rest.starts_with("* ") || rest.starts_with("+ ") {
        return Some((columns, false, rest));
    }
    let mut digits = 0usize;
    while digits < rest.len() && rest.as_bytes()[digits].is_ascii_digit() {
        digits += 1;
    }
    if digits == 0 || digits > 9 {
        return None;
    }
    let after_digits = rest.as_bytes().get(digits).copied()?;
    if after_digits != b'.' && after_digits != b')' {
        return None;
    }
    if rest.as_bytes().get(digits + 1).copied() != Some(b' ') {
        return None;
    }
    Some((columns, true, &rest[digits + 2..]))
}

fn parse_blocks<'a>(parent: &'a AstNode<'a>) -> Result<Vec<Block>> {
    parent.children().map(parse_block).collect()
}

fn parse_block<'a>(node: &'a AstNode<'a>) -> Result<Block> {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Paragraph => Ok(Block::Paragraph {
            content: parse_inlines(node)?,
        }),
        NodeValue::Heading(heading) => Ok(Block::Heading {
            level: heading.level,
            content: parse_inlines(node)?,
        }),
        NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => Ok(Block::BlockQuote {
            blocks: parse_blocks(node)?,
        }),
        NodeValue::CodeBlock(code) => {
            let language = code
                .info
                .split_whitespace()
                .next()
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            Ok(Block::CodeBlock {
                language,
                text: code.literal.clone(),
            })
        }
        NodeValue::List(list) => {
            let items = node
                .children()
                .map(|item| {
                    let checked = item.descendants().find_map(|descendant| {
                        match &descendant.data.borrow().value {
                            NodeValue::TaskItem(task) => Some(task.symbol.is_some()),
                            _ => None,
                        }
                    });
                    Ok(ListItem {
                        checked,
                        blocks: parse_blocks(item)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Block::List {
                ordered: list.list_type == ListType::Ordered,
                start: list.start as u64,
                tight: list.tight,
                items,
            })
        }
        NodeValue::Table(table) => parse_table(node, &table.alignments),
        NodeValue::ThematicBreak => Ok(Block::ThematicBreak),
        NodeValue::FrontMatter(raw) => Ok(Block::FrontMatter { raw: raw.clone() }),
        NodeValue::HtmlBlock(html) => Ok(Block::Html {
            raw: html.literal.clone(),
        }),
        NodeValue::FootnoteDefinition(footnote) => Ok(Block::FootnoteDefinition {
            name: footnote.name.clone(),
            blocks: parse_blocks(node)?,
        }),
        value => Err(Error::UnsupportedMarkdownNode(value.xml_node_name())),
    }
}

fn parse_table<'a>(
    table: &'a AstNode<'a>,
    source_alignments: &[ComrakTableAlignment],
) -> Result<Block> {
    let alignments = source_alignments
        .iter()
        .map(|alignment| match alignment {
            ComrakTableAlignment::None => TableAlignment::None,
            ComrakTableAlignment::Left => TableAlignment::Left,
            ComrakTableAlignment::Center => TableAlignment::Center,
            ComrakTableAlignment::Right => TableAlignment::Right,
        })
        .collect();

    let mut header = Vec::new();
    let mut rows = Vec::new();
    for row in table.children() {
        let cells = row
            .children()
            .map(|cell| parse_inlines(cell))
            .collect::<Result<Vec<TableCell>>>()?;

        let is_header = matches!(row.data.borrow().value, NodeValue::TableRow(true));
        if is_header && header.is_empty() {
            header = cells;
        } else {
            rows.push(cells);
        }
    }

    Ok(Block::Table {
        alignments,
        header,
        rows,
    })
}

fn parse_inlines<'a>(parent: &'a AstNode<'a>) -> Result<Vec<Inline>> {
    parent.children().map(parse_inline).collect()
}

fn parse_inline<'a>(node: &'a AstNode<'a>) -> Result<Inline> {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(value) => Ok(Inline::Text {
            value: value.to_string(),
        }),
        NodeValue::SoftBreak => Ok(Inline::SoftBreak),
        NodeValue::LineBreak => Ok(Inline::HardBreak),
        NodeValue::Emph => Ok(Inline::Emphasis {
            content: parse_inlines(node)?,
        }),
        NodeValue::Strong => Ok(Inline::Strong {
            content: parse_inlines(node)?,
        }),
        NodeValue::Strikethrough => Ok(Inline::Strikethrough {
            content: parse_inlines(node)?,
        }),
        NodeValue::Code(code) => Ok(Inline::Code {
            value: code.literal.clone(),
        }),
        NodeValue::Math(math) => Ok(Inline::Math {
            display: math.display_math,
            value: math.literal.clone(),
        }),
        NodeValue::Link(link) => Ok(Inline::Link {
            url: link.url.clone(),
            title: link.title.clone(),
            content: parse_inlines(node)?,
        }),
        NodeValue::Image(image) => Ok(Inline::Image {
            url: image.url.clone(),
            title: image.title.clone(),
            alt: parse_inlines(node)?,
        }),
        NodeValue::HtmlInline(raw) | NodeValue::Raw(raw) => Ok(Inline::Html { raw: raw.clone() }),
        NodeValue::FootnoteReference(footnote) => Ok(Inline::FootnoteReference {
            name: footnote.name.clone(),
        }),
        // The task marker is represented by ListItem::checked. Keeping it out
        // of the inline stream prevents duplicate checkboxes during export.
        NodeValue::TaskItem(_) => Ok(Inline::Text {
            value: String::new(),
        }),
        value => Err(Error::UnsupportedMarkdownNode(value.xml_node_name())),
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::model::{Block, Inline, TableAlignment};

    #[test]
    fn parses_commonmark_and_gfm_structure() {
        let source = concat!(
            "---\ntitle: Demo\n---\n\n",
            "# Heading\n\n",
            "Text with **bold**, *italic*, `code`, and ~~removed~~.\n\n",
            "- [x] done\n- [ ] open\n\n",
            "| Left | Right |\n| :--- | ---: |\n| A | B |\n",
        );

        let document = parse(source).expect("Markdown should parse");
        assert!(matches!(document.blocks[0], Block::FrontMatter { .. }));
        assert!(matches!(
            document.blocks[1],
            Block::Heading { level: 1, .. }
        ));

        let Block::Paragraph { content } = &document.blocks[2] else {
            panic!("expected paragraph");
        };
        assert!(
            content
                .iter()
                .any(|inline| matches!(inline, Inline::Strong { .. }))
        );
        assert!(
            content
                .iter()
                .any(|inline| matches!(inline, Inline::Strikethrough { .. }))
        );

        let Block::List { items, .. } = &document.blocks[3] else {
            panic!("expected task list");
        };
        assert_eq!(items[0].checked, Some(true));
        assert_eq!(items[1].checked, Some(false));

        let Block::Table { alignments, .. } = &document.blocks[4] else {
            panic!("expected table");
        };
        assert_eq!(alignments, &[TableAlignment::Left, TableAlignment::Right]);
    }

    #[test]
    fn parses_math_and_footnotes() {
        let document = parse("Formula $x^2$.[^n]\n\n[^n]: Note").expect("Markdown should parse");
        let Block::Paragraph { content } = &document.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            content
                .iter()
                .any(|inline| matches!(inline, Inline::Math { .. }))
        );
        assert!(
            content
                .iter()
                .any(|inline| matches!(inline, Inline::FootnoteReference { .. }))
        );
        assert!(matches!(
            document.blocks[1],
            Block::FootnoteDefinition { .. }
        ));
    }

    #[test]
    fn nests_ordered_lists_with_two_space_indent() {
        let document = parse("1. one\n  2. two\n  3. three\n")
            .expect("ordered list with 2-space indent should nest");
        let Block::List {
            ordered: true,
            items,
            ..
        } = &document.blocks[0]
        else {
            panic!("expected ordered list, got {:?}", document.blocks);
        };
        assert_eq!(items.len(), 1, "outer list should keep a single parent item");
        let nested = items[0]
            .blocks
            .iter()
            .find_map(|block| match block {
                Block::List {
                    ordered: true,
                    items,
                    ..
                } => Some(items),
                _ => None,
            })
            .expect("level-1 nested ordered list");
        assert_eq!(nested.len(), 2, "two nested items at one indent level");
    }

    #[test]
    fn starts_ordered_list_at_one_after_bullets() {
        let document = parse("- a\n- b\n\n1. c\n1. d\n")
            .expect("bullet then ordered should parse");
        assert!(matches!(
            &document.blocks[0],
            Block::List {
                ordered: false,
                ..
            }
        ));
        let Block::List {
            ordered: true,
            start,
            items,
            ..
        } = &document.blocks[1]
        else {
            panic!("expected ordered list after bullets, got {:?}", document.blocks);
        };
        assert_eq!(*start, 1, "ordered list must not continue bullet counters");
        assert_eq!(items.len(), 2);
    }
}
