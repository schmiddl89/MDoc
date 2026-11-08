pub mod docx;
pub mod docx_reader;
pub mod error;
pub mod markdown;
pub mod markdown_writer;
pub mod model;
pub mod paths;
pub mod style;
pub mod template;

pub use docx::ExportOptions;
pub use error::{Error, Result};
pub use model::{Block, Document, Inline, ListItem, TableAlignment, TableCell};
pub use paths::{
    TemplateFileFingerprint, config_dir, ensure_shared_template_styles_cache,
    load_shared_template_path, normal_dotm_path, resolve_template, save_shared_template_path,
    shared_style_profile_path, shared_template_numbering_path, shared_template_path_file,
    shared_template_styles_meta_path, shared_template_styles_path,
};
pub use style::{
    CharacterStyleMap, ParagraphStyleMap, StyleProfile, migrate_legacy_english_ids,
    profile_from_template,
};

#[must_use]
pub fn document_to_markdown(document: &Document) -> String {
    markdown_writer::write(document)
}

pub fn docx_to_document(
    input: impl AsRef<std::path::Path>,
    profile: &StyleProfile,
) -> Result<Document> {
    docx_reader::read(input, profile)
}

pub fn docx_to_markdown(
    input: impl AsRef<std::path::Path>,
    profile: &StyleProfile,
) -> Result<String> {
    let document = docx_to_document(input, profile)?;
    Ok(document_to_markdown(&document))
}

pub fn markdown_to_docx(markdown: &str, output: impl AsRef<std::path::Path>) -> Result<Document> {
    markdown_to_docx_with_options(markdown, output, &ExportOptions::default())
}

pub fn markdown_to_docx_with_options(
    markdown: &str,
    output: impl AsRef<std::path::Path>,
    options: &ExportOptions,
) -> Result<Document> {
    let document = markdown::parse(markdown)?;
    docx::write_with_options(&document, output, options)?;
    Ok(document)
}
