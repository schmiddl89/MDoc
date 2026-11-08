use std::fmt::Write as _;

use crate::model::{Block, Document, Inline, ListItem, TableAlignment};

#[must_use]
pub fn write(document: &Document) -> String {
    render_blocks(&document.blocks)
}

fn render_blocks(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(render_block)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_block(block: &Block) -> String {
    match block {
        Block::Paragraph { content } => render_inlines(content),
        Block::Heading { level, content } => {
            format!(
                "{} {}",
                "#".repeat(usize::from(*level)),
                render_inlines(content)
            )
        }
        Block::BlockQuote { blocks } => render_blocks(blocks)
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        Block::CodeBlock { language, text } => {
            let fence = code_fence(text);
            let language = language.as_deref().unwrap_or_default();
            format!(
                "{fence}{language}\n{}\n{fence}",
                text.trim_end_matches('\n')
            )
        }
        Block::List {
            ordered,
            start,
            items,
            ..
        } => render_list(*ordered, *start, items),
        Block::Table {
            alignments,
            header,
            rows,
        } => render_table(alignments, header, rows),
        Block::ThematicBreak => "---".to_owned(),
        Block::FrontMatter { raw } | Block::Html { raw } => raw.trim_end().to_owned(),
        Block::FootnoteDefinition { name, blocks } => {
            let body = render_blocks(blocks);
            let mut lines = body.lines();
            let first = lines.next().unwrap_or_default();
            let mut output = format!("[^{name}]: {first}");
            for line in lines {
                let _ = write!(output, "\n    {line}");
            }
            output
        }
    }
}

fn render_list(ordered: bool, start: u64, items: &[ListItem]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = if ordered {
                format!("{}. ", start + index as u64)
            } else {
                "- ".to_owned()
            };
            let task = match item.checked {
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            };
            let body = render_blocks(&item.blocks);
            let continuation = " ".repeat(marker.len());
            let mut lines = body.lines();
            let first = lines.next().unwrap_or_default();
            let mut output = format!("{marker}{task}{first}");
            for line in lines {
                let _ = write!(output, "\n{continuation}{line}");
            }
            output
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_table(
    alignments: &[TableAlignment],
    header: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
) -> String {
    let column_count = header
        .len()
        .max(rows.iter().map(Vec::len).max().unwrap_or_default());
    if column_count == 0 {
        return String::new();
    }

    let mut output = String::new();
    output.push_str(&table_row(header, column_count));
    output.push('\n');
    output.push('|');
    for index in 0..column_count {
        let alignment = alignments.get(index).copied().unwrap_or_default();
        let marker = match alignment {
            TableAlignment::None => "---",
            TableAlignment::Left => ":---",
            TableAlignment::Center => ":---:",
            TableAlignment::Right => "---:",
        };
        let _ = write!(output, " {marker} |");
    }
    for row in rows {
        output.push('\n');
        output.push_str(&table_row(row, column_count));
    }
    output
}

fn table_row(cells: &[Vec<Inline>], column_count: usize) -> String {
    let mut output = String::from("|");
    for index in 0..column_count {
        let value = cells.get(index).map_or_else(String::new, |cell| {
            render_inlines(cell).replace('|', "\\|").replace('\n', " ")
        });
        let _ = write!(output, " {value} |");
    }
    output
}

fn render_inlines(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { value } => output.push_str(&escape_text(value)),
            Inline::SoftBreak => output.push('\n'),
            Inline::HardBreak => output.push_str("  \n"),
            Inline::Emphasis { content } => {
                let _ = write!(output, "*{}*", render_inlines(content));
            }
            Inline::Strong { content } => {
                let _ = write!(output, "**{}**", render_inlines(content));
            }
            Inline::Strikethrough { content } => {
                let _ = write!(output, "~~{}~~", render_inlines(content));
            }
            Inline::Code { value } => {
                let fence = inline_code_fence(value);
                let padding = if value.starts_with('`') || value.ends_with('`') {
                    " "
                } else {
                    ""
                };
                let _ = write!(output, "{fence}{padding}{value}{padding}{fence}");
            }
            Inline::Math { display, value } => {
                let delimiter = if *display { "$$" } else { "$" };
                let _ = write!(output, "{delimiter}{value}{delimiter}");
            }
            Inline::Link {
                url,
                title,
                content,
            } => {
                let title = if title.is_empty() {
                    String::new()
                } else {
                    format!(" \"{}\"", title.replace('"', "\\\""))
                };
                let _ = write!(
                    output,
                    "[{}]({}{title})",
                    render_inlines(content),
                    url.replace(')', "\\)")
                );
            }
            Inline::Image { url, title, alt } => {
                let title = if title.is_empty() {
                    String::new()
                } else {
                    format!(" \"{}\"", title.replace('"', "\\\""))
                };
                let _ = write!(
                    output,
                    "![{}]({}{title})",
                    render_inlines(alt),
                    url.replace(')', "\\)")
                );
            }
            Inline::Html { raw } => output.push_str(raw),
            Inline::FootnoteReference { name } => {
                let _ = write!(output, "[^{name}]");
            }
        }
    }
    output
}

fn escape_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '*' | '_' | '[' | ']' | '`' | '~') {
            output.push('\\');
        }
        output.push(character);
    }
    output
}

fn inline_code_fence(value: &str) -> String {
    let longest = longest_run(value, '`');
    "`".repeat(longest + 1)
}

fn code_fence(value: &str) -> String {
    let longest = longest_run(value, '`');
    "`".repeat((longest + 1).max(3))
}

fn longest_run(value: &str, needle: char) -> usize {
    value
        .chars()
        .fold((0, 0), |(longest, current), character| {
            if character == needle {
                (longest.max(current + 1), current + 1)
            } else {
                (longest, 0)
            }
        })
        .0
}

#[cfg(test)]
mod tests {
    use super::write;
    use crate::markdown;

    #[test]
    fn serializes_commonmark_and_gfm_constructs() {
        let source = concat!(
            "# Heading\n\n",
            "Text with **bold**, *italic*, `code`, and ~~removed~~.\n\n",
            "- [x] done\n- [ ] open\n\n",
            "| Left | Right |\n| :--- | ---: |\n| A | B |\n",
        );
        let document = markdown::parse(source).expect("fixture should parse");
        let rendered = write(&document);
        let reparsed = markdown::parse(&rendered).expect("rendered Markdown should parse");

        assert_eq!(reparsed, document);
    }
}
