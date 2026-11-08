use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use mdoc_core::template::TemplateStyleKind;
use mdoc_core::{ExportOptions, StyleProfile, docx, markdown, template};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const FEATURE_SMOKE: &str = include_str!("../../../fixtures/feature-smoke.md");

#[test]
fn exports_a_valid_docx_package_with_expected_structure() {
    let document = markdown::parse(FEATURE_SMOKE).expect("fixture should parse");
    let mut output = Cursor::new(Vec::new());

    docx::build(&document)
        .build()
        .pack(&mut output)
        .expect("DOCX should be written");

    let mut archive =
        ZipArchive::new(Cursor::new(output.into_inner())).expect("DOCX should be a ZIP package");
    for required_part in [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/document.xml",
        "word/styles.xml",
        "word/_rels/document.xml.rels",
    ] {
        archive
            .by_name(required_part)
            .unwrap_or_else(|_| panic!("missing DOCX part: {required_part}"));
    }

    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("document XML should exist")
        .read_to_string(&mut document_xml)
        .expect("document XML should be UTF-8");

    assert!(document_xml.contains("mdoc"));
    assert!(document_xml.contains("berschrift1"));
    assert!(document_xml.contains("CommonMark"));
    assert!(document_xml.contains("E = mc^2"));
    assert!(document_xml.contains("w:numPr"));
    assert!(document_xml.contains("\u{2612}"));
    assert!(document_xml.contains("\u{2610}"));
    assert!(
        !document_xml.contains("w:outlineLvl"),
        "headings must rely on style outline, not paragraph outlineLvl"
    );

    let mut numbering_xml = String::new();
    archive
        .by_name("word/numbering.xml")
        .expect("numbering XML should exist")
        .read_to_string(&mut numbering_xml)
        .expect("numbering XML should be UTF-8");
    assert!(numbering_xml.contains("bullet"));
    assert!(numbering_xml.contains("\u{2022}"));
    assert!(
        numbering_xml.contains("w:abstractNumId=\"1000\"")
            || numbering_xml.contains("w:numId=\"1000\""),
        "list numbering must use high IDs to avoid template collisions"
    );

    let mut relationships = String::new();
    archive
        .by_name("word/_rels/document.xml.rels")
        .expect("document relationships should exist")
        .read_to_string(&mut relationships)
        .expect("relationships XML should be UTF-8");
    assert!(relationships.contains("https://example.com"));
}

#[test]
fn applies_template_styles_and_semantic_style_mapping() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    let template_path = std::env::temp_dir().join(format!(
        "mdoc-template-{}-{nonce}.dotx",
        std::process::id()
    ));
    let output_path =
        std::env::temp_dir().join(format!("mdoc-output-{}-{nonce}.docx", std::process::id()));
    let styles_xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
        r#"<w:style w:type="paragraph" w:styleId="CompanyHeading">"#,
        r#"<w:name w:val="Company Heading"/></w:style></w:styles>"#,
    );

    let template_file = File::create(&template_path).expect("template should be created");
    let mut template_zip = ZipWriter::new(template_file);
    template_zip
        .start_file("word/styles.xml", SimpleFileOptions::default())
        .expect("styles part should be created");
    template_zip
        .write_all(styles_xml.as_bytes())
        .expect("styles XML should be written");
    template_zip.finish().expect("template should be finalized");

    let styles = template::inspect(&template_path).expect("template styles should be readable");
    assert_eq!(styles.len(), 1);
    assert_eq!(styles[0].id, "CompanyHeading");
    assert_eq!(styles[0].name, "Company Heading");
    assert_eq!(styles[0].kind, TemplateStyleKind::Paragraph);

    let mut profile = StyleProfile::default();
    profile.paragraphs.heading1 = Some("CompanyHeading".to_owned());
    let options = ExportOptions {
        style_profile: profile,
        template: Some(template_path.clone()),
    };
    mdoc_core::markdown_to_docx_with_options("# Template heading", &output_path, &options)
        .expect("templated DOCX should be exported");

    let output_file = File::open(&output_path).expect("output should exist");
    let mut output_zip = ZipArchive::new(output_file).expect("output should be a DOCX package");
    let mut exported_styles = String::new();
    output_zip
        .by_name("word/styles.xml")
        .expect("styles part should exist")
        .read_to_string(&mut exported_styles)
        .expect("styles part should be UTF-8");
    assert!(
        exported_styles.contains("CompanyHeading"),
        "template styles must be preserved"
    );
    assert!(
        exported_styles.contains("Company Heading"),
        "template style names must be preserved"
    );

    let mut document_xml = String::new();
    output_zip
        .by_name("word/document.xml")
        .expect("document part should exist")
        .read_to_string(&mut document_xml)
        .expect("document part should be UTF-8");
    assert!(document_xml.contains("CompanyHeading"));

    fs::remove_file(template_path).expect("template should be removed");
    fs::remove_file(output_path).expect("output should be removed");
}

#[test]
fn preserves_template_heading_numbering_while_keeping_markdown_lists() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    let template_path = std::env::temp_dir().join(format!(
        "mdoc-outline-template-{}-{nonce}.dotx",
        std::process::id()
    ));
    let output_path = std::env::temp_dir().join(format!(
        "mdoc-outline-output-{}-{nonce}.docx",
        std::process::id()
    ));

    let styles_xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
        r#"<w:style w:type="paragraph" w:styleId="berschrift1">"#,
        r#"<w:name w:val="Überschrift 1"/>"#,
        r#"<w:pPr><w:outlineLvl w:val="0"/><w:numPr><w:numId w:val="2"/></w:numPr></w:pPr>"#,
        r#"</w:style>"#,
        r#"<w:style w:type="paragraph" w:styleId="berschrift2">"#,
        r#"<w:name w:val="Überschrift 2"/>"#,
        r#"<w:pPr><w:outlineLvl w:val="1"/><w:numPr><w:numId w:val="2"/></w:numPr></w:pPr>"#,
        r#"</w:style></w:styles>"#,
    );
    let numbering_xml = concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
        r#"<w:abstractNum w:abstractNumId="0">"#,
        r#"<w:multiLevelType w:val="multilevel"/>"#,
        r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/>"#,
        r#"<w:pStyle w:val="berschrift1"/><w:lvlText w:val="%1"/><w:lvlJc w:val="left"/></w:lvl>"#,
        r#"<w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="decimal"/>"#,
        r#"<w:pStyle w:val="berschrift2"/><w:lvlText w:val="%1.%2"/><w:lvlJc w:val="left"/></w:lvl>"#,
        r#"</w:abstractNum>"#,
        r#"<w:num w:numId="2"><w:abstractNumId w:val="0"/></w:num>"#,
        r#"</w:numbering>"#,
    );

    let template_file = File::create(&template_path).expect("template should be created");
    let mut template_zip = ZipWriter::new(template_file);
    template_zip
        .start_file("word/styles.xml", SimpleFileOptions::default())
        .expect("styles part");
    template_zip
        .write_all(styles_xml.as_bytes())
        .expect("styles xml");
    template_zip
        .start_file("word/numbering.xml", SimpleFileOptions::default())
        .expect("numbering part");
    template_zip
        .write_all(numbering_xml.as_bytes())
        .expect("numbering xml");
    template_zip.finish().expect("template finalize");

    let profile = mdoc_core::profile_from_template(&template_path).expect("auto profile");
    assert_eq!(profile.paragraphs.heading1.as_deref(), Some("berschrift1"));
    assert_eq!(profile.paragraphs.heading2.as_deref(), Some("berschrift2"));

    let options = ExportOptions {
        style_profile: profile,
        template: Some(template_path.clone()),
    };
    mdoc_core::markdown_to_docx_with_options(
        "# Titel\n\n## Untertitel\n\n- Punkt A\n- Punkt B\n",
        &output_path,
        &options,
    )
    .expect("export");

    let output_file = File::open(&output_path).expect("output");
    let mut output_zip = ZipArchive::new(output_file).expect("zip");

    let mut exported_numbering = String::new();
    output_zip
        .by_name("word/numbering.xml")
        .expect("numbering")
        .read_to_string(&mut exported_numbering)
        .expect("utf8");
    assert!(
        exported_numbering.contains("w:numId=\"2\""),
        "template heading numId 2 must survive"
    );
    assert!(
        exported_numbering.contains("berschrift1"),
        "template outline pStyle must survive"
    );
    assert!(
        exported_numbering.contains("%1.%2"),
        "template multilevel text must survive"
    );
    assert!(
        exported_numbering.contains("w:numId=\"1000\"")
            || exported_numbering.contains("abstractNumId=\"1000\""),
        "markdown list numbering must be merged with high IDs"
    );

    let mut document_xml = String::new();
    output_zip
        .by_name("word/document.xml")
        .expect("document")
        .read_to_string(&mut document_xml)
        .expect("utf8");
    assert!(document_xml.contains("berschrift1"));
    assert!(document_xml.contains("berschrift2"));
    assert!(
        !document_xml.contains("w:outlineLvl"),
        "must not override style outline on paragraphs"
    );

    let mut exported_styles = String::new();
    output_zip
        .by_name("word/styles.xml")
        .expect("styles")
        .read_to_string(&mut exported_styles)
        .expect("utf8");
    assert!(
        exported_styles.contains("Listenabsatz") || exported_styles.contains("numId"),
        "list paragraph styles should carry numbering for template reuse"
    );

    fs::remove_file(template_path).expect("cleanup template");
    fs::remove_file(output_path).expect("cleanup output");
}

#[test]
fn list_paragraphs_reference_mapped_styles_without_stub_defs() {
    let document = markdown::parse("- eins\n- zwei\n\n1. a\n2. b\n").expect("parse");
    let mut profile = StyleProfile::default();
    profile.paragraphs.unordered_list = Some("Aufzhlung".to_owned());
    profile.paragraphs.ordered_list = Some("Nummerierung".to_owned());

    let mut output = Cursor::new(Vec::new());
    docx::build_with_profile(&document, &profile)
        .build()
        .pack(&mut output)
        .expect("pack");

    let mut archive =
        ZipArchive::new(Cursor::new(output.into_inner())).expect("zip");
    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("document")
        .read_to_string(&mut document_xml)
        .expect("utf8");
    assert!(document_xml.contains("Aufzhlung"));
    assert!(document_xml.contains("Nummerierung"));
    assert!(document_xml.contains("w:numPr"));

    let mut styles_xml = String::new();
    archive
        .by_name("word/styles.xml")
        .expect("styles")
        .read_to_string(&mut styles_xml)
        .expect("utf8");
    // Keine Listen-Style-Stubs — sonst legt FormattedText „Aufzhlung“ neben „Aufzählung“ an.
    assert!(!styles_xml.contains("styleId=\"Aufzhlung\""));
    assert!(!styles_xml.contains("styleId=\"Nummerierung\""));
    assert!(!styles_xml.contains("w:val=\"List Paragraph\""));
    assert!(!styles_xml.contains("w:val=\"Emphasis\""));
}
