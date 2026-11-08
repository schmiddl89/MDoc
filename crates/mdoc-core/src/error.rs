use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the Markdown document contains an unsupported node: {0}")]
    UnsupportedMarkdownNode(&'static str),

    #[error("could not create DOCX file at {path}: {source}")]
    CreateDocx {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not write DOCX package: {0}")]
    WriteDocx(String),

    #[error("could not open Word template at {path}: {source}")]
    OpenTemplate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not read Word template at {path}: {source}")]
    ReadTemplate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid Word template at {path}: {message}")]
    InvalidTemplate { path: PathBuf, message: String },

    #[error("could not read DOCX file at {path}: {source}")]
    ReadDocxFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not parse DOCX package: {0}")]
    ParseDocx(String),
}

pub type Result<T> = std::result::Result<T, Error>;
