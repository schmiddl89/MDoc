use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub metadata: BTreeMap<String, String>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Paragraph {
        content: Vec<Inline>,
    },
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    BlockQuote {
        blocks: Vec<Block>,
    },
    CodeBlock {
        language: Option<String>,
        text: String,
    },
    List {
        ordered: bool,
        start: u64,
        tight: bool,
        items: Vec<ListItem>,
    },
    Table {
        alignments: Vec<TableAlignment>,
        header: Vec<TableCell>,
        rows: Vec<Vec<TableCell>>,
    },
    ThematicBreak,
    FrontMatter {
        raw: String,
    },
    Html {
        raw: String,
    },
    FootnoteDefinition {
        name: String,
        blocks: Vec<Block>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub blocks: Vec<Block>,
}

pub type TableCell = Vec<Inline>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TableAlignment {
    #[default]
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Text {
        value: String,
    },
    SoftBreak,
    HardBreak,
    Emphasis {
        content: Vec<Inline>,
    },
    Strong {
        content: Vec<Inline>,
    },
    Strikethrough {
        content: Vec<Inline>,
    },
    Code {
        value: String,
    },
    Math {
        display: bool,
        value: String,
    },
    Link {
        url: String,
        title: String,
        content: Vec<Inline>,
    },
    Image {
        url: String,
        title: String,
        alt: Vec<Inline>,
    },
    Html {
        raw: String,
    },
    FootnoteReference {
        name: String,
    },
}

impl Document {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

impl Inline {
    pub(crate) fn append_plain_text(&self, output: &mut String) {
        match self {
            Self::Text { value } | Self::Code { value } | Self::Math { value, .. } => {
                output.push_str(value);
            }
            Self::SoftBreak | Self::HardBreak => output.push('\n'),
            Self::Emphasis { content }
            | Self::Strong { content }
            | Self::Strikethrough { content }
            | Self::Link { content, .. } => {
                for inline in content {
                    inline.append_plain_text(output);
                }
            }
            Self::Image { alt, .. } => {
                for inline in alt {
                    inline.append_plain_text(output);
                }
            }
            Self::Html { raw } => output.push_str(raw),
            Self::FootnoteReference { name } => {
                output.push_str("[^");
                output.push_str(name);
                output.push(']');
            }
        }
    }
}

pub(crate) fn inline_plain_text(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        inline.append_plain_text(&mut output);
    }
    output
}
