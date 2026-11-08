use std::io::Cursor;

use mdoc_core::model::{Block, Inline};
use mdoc_core::{StyleProfile, document_to_markdown, docx, docx_reader, markdown};

const ROUNDTRIP_MARKDOWN: &str = r#"# Heading

Text with **bold**, *italic*, ~~removed~~, `code`, and [a link](https://example.com).

- [x] done
- [ ] open

1. first
2. second

> quoted text

```rust
fn main() {}
```

| Left | Right |
| :--- | ---: |
| A | B |
"#;
const FEATURE_SMOKE: &str = include_str!("../../../fixtures/feature-smoke.md");

#[test]
fn markdown_docx_markdown_preserves_core_semantics() {
    let source = markdown::parse(ROUNDTRIP_MARKDOWN).expect("Markdown fixture should parse");
    let mut package = Cursor::new(Vec::new());
    docx::build(&source)
        .build()
        .pack(&mut package)
        .expect("DOCX should be generated");

    let imported = docx_reader::from_bytes(&package.into_inner(), &StyleProfile::default())
        .expect("DOCX should import");
    let rendered = document_to_markdown(&imported);
    let reparsed = markdown::parse(&rendered).expect("imported Markdown should parse");

    assert!(matches!(
        reparsed.blocks.first(),
        Some(Block::Heading { level: 1, .. })
    ));
    assert!(reparsed.blocks.iter().any(|block| matches!(
        block,
        Block::List {
            ordered: false,
            items,
            ..
        } if items.iter().any(|item| item.checked == Some(true))
    )));
    assert!(reparsed.blocks.iter().any(|block| matches!(
        block,
        Block::List {
            ordered: true,
            items,
            ..
        } if items.len() == 2
    )));
    assert!(
        reparsed
            .blocks
            .iter()
            .any(|block| matches!(block, Block::BlockQuote { .. }))
    );
    assert!(reparsed.blocks.iter().any(|block| matches!(
        block,
        Block::CodeBlock {
            language: Some(language),
            text,
        } if language == "rust" && text.contains("fn main")
    )));
    assert!(
        reparsed
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Table { .. }))
    );

    let paragraph = reparsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph { content } => Some(content),
            _ => None,
        })
        .expect("paragraph should survive");
    assert!(
        paragraph
            .iter()
            .any(|inline| matches!(inline, Inline::Strong { .. }))
    );
    assert!(
        paragraph
            .iter()
            .any(|inline| matches!(inline, Inline::Emphasis { .. }))
    );
    assert!(
        paragraph
            .iter()
            .any(|inline| matches!(inline, Inline::Strikethrough { .. }))
    );
    assert!(
        paragraph
            .iter()
            .any(|inline| matches!(inline, Inline::Code { .. }))
    );
    assert!(
        paragraph.iter().any(
            |inline| matches!(inline, Inline::Link { url, .. } if url == "https://example.com")
        )
    );
}

#[test]
fn roundtrip_preserves_frontmatter_quotes_tables_and_footnotes() {
    let source = markdown::parse(FEATURE_SMOKE).expect("feature fixture should parse");
    let mut package = Cursor::new(Vec::new());
    docx::build(&source)
        .build()
        .pack(&mut package)
        .expect("DOCX should be generated");

    let imported = docx_reader::from_bytes(&package.into_inner(), &StyleProfile::default())
        .expect("DOCX should import");
    let reparsed =
        markdown::parse(&document_to_markdown(&imported)).expect("imported Markdown should parse");

    assert!(matches!(
        reparsed.blocks.first(),
        Some(Block::FrontMatter { .. })
    ));
    assert!(
        reparsed
            .blocks
            .iter()
            .any(|block| matches!(block, Block::Table { .. }))
    );
    assert!(reparsed.blocks.iter().any(|block| matches!(
        block,
        Block::BlockQuote { blocks }
            if matches!(
                blocks.as_slice(),
                [Block::Paragraph { content }]
                    if content.iter().any(|inline| matches!(inline, Inline::Strong { .. }))
            )
    )));
    assert!(reparsed.blocks.iter().any(|block| matches!(
        block,
        Block::FootnoteDefinition { name, .. } if name == "note"
    )));
}
