use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::docx::LIST_NUMBERING_ID_BASE;
use crate::error::{Error, Result};
use crate::style::StyleProfile;

const STYLES_PART: &str = "word/styles.xml";
const NUMBERING_PART: &str = "word/numbering.xml";
const SUPPORTING_STYLE_PARTS: [&str; 1] = ["word/fontTable.xml"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateStyle {
    pub id: String,
    pub name: String,
    pub kind: TemplateStyleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TemplateStyleKind {
    Paragraph,
    Character,
    Table,
    Numbering,
    Other,
}

pub fn inspect(path: impl AsRef<Path>) -> Result<Vec<TemplateStyle>> {
    let path = path.as_ref();
    let styles_xml = read_styles_part(path)?;
    parse_styles(path, &styles_xml)
}

/// Kopiert `word/styles.xml` aus einer Vorlage (Share-Read, auch bei Word-Sperre wenn möglich).
pub fn cache_styles_xml(template_path: &Path, output_path: &Path) -> Result<()> {
    let styles = read_styles_part(template_path)?;
    write_bytes(output_path, &styles)
}

/// Cached `styles.xml` und optional `numbering.xml` (Outline-Nummerierung der Überschriften).
pub fn cache_template_parts(
    template_path: &Path,
    styles_output: &Path,
    numbering_output: &Path,
) -> Result<()> {
    cache_styles_xml(template_path, styles_output)?;
    match read_optional_part(template_path, NUMBERING_PART)? {
        Some(numbering) => write_bytes(numbering_output, &numbering)?,
        None => {
            if numbering_output.is_file() {
                fs::remove_file(numbering_output).map_err(|source| Error::ReadTemplate {
                    path: numbering_output.to_owned(),
                    source,
                })?;
            }
        }
    }
    Ok(())
}

fn write_bytes(output_path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::ReadTemplate {
            path: output_path.to_owned(),
            source,
        })?;
    }
    fs::write(output_path, bytes).map_err(|source| Error::ReadTemplate {
        path: output_path.to_owned(),
        source,
    })
}

/// Builds a style profile from template display names (DE/EN).
pub fn infer_profile(path: impl AsRef<Path>) -> Result<StyleProfile> {
    crate::style::profile_from_template(path)
}

/// Öffnet eine .dotx/.dotm/.docx auch bei Word-Dateisperre (FILE_SHARE_READ|WRITE).
fn open_template_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
        options.share_mode(0x0000_0007);
    }
    options.open(path).map_err(|source| Error::OpenTemplate {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn apply_styles<W: Write + Seek>(
    generated_docx: &[u8],
    template_path: &Path,
    output: W,
) -> Result<()> {
    let template_parts = read_style_parts(template_path)?;
    let template_numbering = read_optional_part(template_path, NUMBERING_PART)?;
    let mut generated = ZipArchive::new(Cursor::new(generated_docx))
        .map_err(|error| Error::WriteDocx(error.to_string()))?;
    let mut writer = ZipWriter::new(output);

    for index in 0..generated.len() {
        let mut entry = generated
            .by_index(index)
            .map_err(|error| Error::WriteDocx(error.to_string()))?;
        let name = entry.name().to_owned();
        let options = SimpleFileOptions::default()
            .compression_method(entry.compression())
            .unix_permissions(entry.unix_mode().unwrap_or(0o644));

        if entry.is_dir() {
            writer
                .add_directory(name, options)
                .map_err(|error| Error::WriteDocx(error.to_string()))?;
            continue;
        }

        writer
            .start_file(&name, options)
            .map_err(|error| Error::WriteDocx(error.to_string()))?;

        if name == NUMBERING_PART {
            let mut generated_numbering = Vec::new();
            entry
                .read_to_end(&mut generated_numbering)
                .map_err(|error| Error::WriteDocx(error.to_string()))?;
            let merged = match &template_numbering {
                Some(template_xml) => {
                    merge_list_numberings(template_xml, &generated_numbering, LIST_NUMBERING_ID_BASE)?
                }
                None => generated_numbering,
            };
            writer
                .write_all(&merged)
                .map_err(|error| Error::WriteDocx(error.to_string()))?;
        } else if name == STYLES_PART {
            let mut generated_styles = Vec::new();
            entry
                .read_to_end(&mut generated_styles)
                .map_err(|error| Error::WriteDocx(error.to_string()))?;
            let merged = match template_parts.get(STYLES_PART) {
                Some(template_styles) => {
                    merge_list_style_numbering(template_styles, &generated_styles)?
                }
                None => generated_styles,
            };
            writer
                .write_all(&merged)
                .map_err(|error| Error::WriteDocx(error.to_string()))?;
        } else if let Some(replacement) = template_parts.get(&name) {
            writer
                .write_all(replacement)
                .map_err(|error| Error::WriteDocx(error.to_string()))?;
        } else {
            std::io::copy(&mut entry, &mut writer)
                .map_err(|error| Error::WriteDocx(error.to_string()))?;
        }
    }

    writer
        .finish()
        .map_err(|error| Error::WriteDocx(error.to_string()))?;
    Ok(())
}

/// Übernimmt `numPr` aus generierten Listen-Styles (numId >= 1000) in die Template-
/// `styles.xml`, damit Aufzählungszeichen/Nummern in der Vorlage erhalten bleiben
/// und beim erneuten Verwenden der DOCX als Vorlage greifen.
fn merge_list_style_numbering(
    template_styles: &[u8],
    generated_styles: &[u8],
) -> Result<Vec<u8>> {
    let generated = std::str::from_utf8(generated_styles).map_err(|error| {
        Error::WriteDocx(format!("generated styles are not UTF-8: {error}"))
    })?;
    let patches = extract_list_style_patches(generated, LIST_NUMBERING_ID_BASE)?;
    if patches.is_empty() {
        return Ok(template_styles.to_vec());
    }

    let mut template = String::from_utf8(template_styles.to_vec()).map_err(|error| {
        Error::WriteDocx(format!("template styles are not UTF-8: {error}"))
    })?;

    for (style_id, style_xml) in &patches {
        if template_contains_style_id(&template, style_id) {
            template = inject_num_pr_into_style(&template, style_id, style_xml)?;
        } else {
            template = append_style_before_close(&template, style_xml)?;
        }
    }
    Ok(template.into_bytes())
}

fn template_contains_style_id(styles_xml: &str, style_id: &str) -> bool {
    let needle = format!("w:styleId=\"{style_id}\"");
    styles_xml.contains(&needle) || styles_xml.contains(&format!("styleId=\"{style_id}\""))
}

fn extract_list_style_patches(
    styles_xml: &str,
    min_num_id: usize,
) -> Result<Vec<(String, String)>> {
    let mut patches = Vec::new();
    let open_patterns = ["<w:style ", "<style "];
    let close_patterns = ["</w:style>", "</style>"];

    let mut search_from = 0;
    while let Some(rel) = open_patterns
        .iter()
        .filter_map(|pattern| {
            styles_xml[search_from..]
                .find(pattern)
                .map(|idx| (idx, pattern.len()))
        })
        .min_by_key(|(idx, _)| *idx)
    {
        let start = search_from + rel.0;
        let after_open = start + rel.1;
        let close_rel = close_patterns
            .iter()
            .filter_map(|pattern| {
                styles_xml[after_open..]
                    .find(pattern)
                    .map(|idx| (idx, pattern.len()))
            })
            .min_by_key(|(idx, _)| *idx)
            .ok_or_else(|| Error::WriteDocx("unclosed <w:style> in generated styles".to_owned()))?;
        let end = after_open + close_rel.0 + close_rel.1;
        let element = &styles_xml[start..end];

        if let Some(style_id) = parse_attr_value(element, "styleId") {
            if let Some(num_id) = style_num_id(element) {
                if num_id >= min_num_id {
                    patches.push((style_id, element.to_owned()));
                }
            }
        }
        search_from = end;
    }
    Ok(patches)
}

fn style_num_id(style_xml: &str) -> Option<usize> {
    let num_pr_start = style_xml.find("<w:numPr").or_else(|| style_xml.find("<numPr"))?;
    let fragment = &style_xml[num_pr_start..];
    // numbering.xml: <w:num w:numId="1000"> — styles.xml: <w:numId w:val="1000"/>
    parse_attr_id(fragment, "numId").or_else(|| {
        let tag_start = fragment.find("<w:numId").or_else(|| fragment.find("<numId"))?;
        parse_attr_id(&fragment[tag_start..], "val")
    })
}

fn parse_attr_value(fragment: &str, attr: &str) -> Option<String> {
    for prefix in [format!("w:{attr}=\""), format!("{attr}=\"")] {
        if let Some(idx) = fragment.find(&prefix) {
            let rest = &fragment[idx + prefix.len()..];
            let end = rest.find('"')?;
            return Some(rest[..end].to_owned());
        }
    }
    None
}

fn inject_num_pr_into_style(styles_xml: &str, style_id: &str, source_style: &str) -> Result<String> {
    let num_pr = extract_num_pr_element(source_style).ok_or_else(|| {
        Error::WriteDocx(format!("list style {style_id} missing numPr"))
    })?;

    let (start, end) = find_style_element_range(styles_xml, style_id).ok_or_else(|| {
        Error::WriteDocx(format!("template style {style_id} not found for numPr merge"))
    })?;
    let style = &styles_xml[start..end];
    let updated = if style_num_id(style).is_some() {
        replace_num_pr_in_style(style, &num_pr)
    } else if let Some(ppr_close) = style.find("</w:pPr>").or_else(|| style.find("</pPr>")) {
        let mut out = String::with_capacity(style.len() + num_pr.len());
        out.push_str(&style[..ppr_close]);
        out.push_str(&num_pr);
        out.push_str(&style[ppr_close..]);
        out
    } else if let Some(relative) = style.find('>') {
        let mut out = String::with_capacity(style.len() + num_pr.len() + 16);
        out.push_str(&style[..=relative]);
        out.push_str(&format!("<w:pPr>{num_pr}</w:pPr>"));
        out.push_str(&style[relative + 1..]);
        out
    } else {
        return Err(Error::WriteDocx(format!(
            "cannot inject numPr into style {style_id}"
        )));
    };

    let mut result = String::with_capacity(styles_xml.len() + num_pr.len());
    result.push_str(&styles_xml[..start]);
    result.push_str(&updated);
    result.push_str(&styles_xml[end..]);
    Ok(result)
}

fn replace_num_pr_in_style(style_xml: &str, new_num_pr: &str) -> String {
    let Some(old) = extract_num_pr_element(style_xml) else {
        return style_xml.to_owned();
    };
    style_xml.replacen(&old, new_num_pr, 1)
}

fn extract_num_pr_element(style_xml: &str) -> Option<String> {
    let start = style_xml.find("<w:numPr").or_else(|| style_xml.find("<numPr"))?;
    let after = &style_xml[start..];
    if let Some(idx) = after.find("</w:numPr>") {
        return Some(after[..idx + "</w:numPr>".len()].to_owned());
    }
    if let Some(idx) = after.find("</numPr>") {
        return Some(after[..idx + "</numPr>".len()].to_owned());
    }
    let end = after.find("/>")?;
    Some(after[..end + 2].to_owned())
}

fn find_style_element_range(styles_xml: &str, style_id: &str) -> Option<(usize, usize)> {
    let markers = [
        format!("w:styleId=\"{style_id}\""),
        format!("styleId=\"{style_id}\""),
    ];
    let id_pos = markers
        .iter()
        .filter_map(|marker| styles_xml.find(marker.as_str()))
        .min()?;
    let start = styles_xml[..id_pos].rfind("<w:style").or_else(|| styles_xml[..id_pos].rfind("<style"))?;
    let after = &styles_xml[id_pos..];
    let end_rel = after
        .find("</w:style>")
        .map(|idx| idx + "</w:style>".len())
        .or_else(|| after.find("</style>").map(|idx| idx + "</style>".len()))?;
    Some((start, id_pos + end_rel))
}

fn append_style_before_close(styles_xml: &str, style_xml: &str) -> Result<String> {
    let close = styles_xml
        .rfind("</w:styles>")
        .or_else(|| styles_xml.rfind("</styles>"))
        .ok_or_else(|| Error::WriteDocx("template styles.xml missing </w:styles>".to_owned()))?;
    let mut out = String::with_capacity(styles_xml.len() + style_xml.len() + 2);
    out.push_str(&styles_xml[..close]);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(style_xml);
    if !style_xml.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&styles_xml[close..]);
    Ok(out)
}

/// Keeps template outline numbering intact and appends Markdown list definitions
/// that use IDs >= `list_id_base`.
fn merge_list_numberings(
    template_xml: &[u8],
    generated_xml: &[u8],
    list_id_base: usize,
) -> Result<Vec<u8>> {
    let list_defs = extract_numbering_defs(generated_xml, list_id_base)?;
    if list_defs.is_empty() {
        return Ok(template_xml.to_vec());
    }

    let template = std::str::from_utf8(template_xml)
        .map_err(|error| Error::WriteDocx(format!("template numbering is not UTF-8: {error}")))?;
    let closing = find_numbering_close(template).ok_or_else(|| {
        Error::WriteDocx("template numbering.xml missing </w:numbering>".to_owned())
    })?;

    let mut merged = Vec::with_capacity(template_xml.len() + list_defs.len() + 8);
    merged.extend_from_slice(template[..closing].as_bytes());
    if !merged.last().is_some_and(|byte| *byte == b'\n') {
        merged.push(b'\n');
    }
    merged.extend_from_slice(&list_defs);
    if !list_defs.ends_with(b"\n") {
        merged.push(b'\n');
    }
    merged.extend_from_slice(template[closing..].as_bytes());
    Ok(merged)
}

fn find_numbering_close(xml: &str) -> Option<usize> {
    ["</w:numbering>", "</numbering>"]
        .into_iter()
        .filter_map(|tag| xml.rfind(tag))
        .min()
}

fn extract_numbering_defs(xml: &[u8], min_id: usize) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(xml)
        .map_err(|error| Error::WriteDocx(format!("generated numbering is not UTF-8: {error}")))?;
    let mut output = Vec::new();

    for (tag, id_attr) in [
        ("abstractNum", "abstractNumId"),
        ("num", "numId"),
    ] {
        extract_elements_with_min_id(text, tag, id_attr, min_id, &mut output)?;
    }
    Ok(output)
}

fn extract_elements_with_min_id(
    xml: &str,
    local_tag: &str,
    id_attr: &str,
    min_id: usize,
    output: &mut Vec<u8>,
) -> Result<()> {
    let open_patterns = [format!("<w:{local_tag} "), format!("<{local_tag} ")];
    let close_patterns = [format!("</w:{local_tag}>"), format!("</{local_tag}>")];

    let mut search_from = 0;
    while let Some(rel) = open_patterns
        .iter()
        .filter_map(|pattern| xml[search_from..].find(pattern).map(|idx| (idx, pattern.len())))
        .min_by_key(|(idx, _)| *idx)
    {
        let start = search_from + rel.0;
        let after_open = start + rel.1;
        let id = parse_attr_id(&xml[start..], id_attr).ok_or_else(|| {
            Error::WriteDocx(format!("missing {id_attr} on <{local_tag}>"))
        })?;

        let close_rel = close_patterns
            .iter()
            .filter_map(|pattern| xml[after_open..].find(pattern).map(|idx| (idx, pattern.len())))
            .min_by_key(|(idx, _)| *idx)
            .ok_or_else(|| Error::WriteDocx(format!("unclosed <{local_tag}>")))?;
        let end = after_open + close_rel.0 + close_rel.1;

        if id >= min_id {
            output.extend_from_slice(xml[start..end].as_bytes());
            output.push(b'\n');
        }
        search_from = end;
    }
    Ok(())
}

fn parse_attr_id(fragment: &str, attr: &str) -> Option<usize> {
    for prefix in [format!("w:{attr}=\""), format!("{attr}=\"")] {
        if let Some(idx) = fragment.find(&prefix) {
            let rest = &fragment[idx + prefix.len()..];
            let end = rest.find('"')?;
            return rest[..end].parse().ok();
        }
    }
    None
}

fn read_styles_part(path: &Path) -> Result<Vec<u8>> {
    read_style_parts(path)?
        .remove(STYLES_PART)
        .ok_or_else(|| Error::InvalidTemplate {
            path: path.to_owned(),
            message: format!("missing {STYLES_PART}"),
        })
}

fn read_optional_part(path: &Path, part_name: &str) -> Result<Option<Vec<u8>>> {
    let file = open_template_file(path)?;
    let mut archive = ZipArchive::new(file).map_err(|error| Error::InvalidTemplate {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    match archive.by_name(part_name) {
        Ok(mut part) => {
            let mut contents = Vec::new();
            part.read_to_end(&mut contents)
                .map_err(|source| Error::ReadTemplate {
                    path: path.to_owned(),
                    source,
                })?;
            Ok(Some(contents))
        }
        Err(_) => Ok(None),
    }
}

fn read_style_parts(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let file = open_template_file(path)?;
    let mut archive = ZipArchive::new(file).map_err(|error| Error::InvalidTemplate {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    let mut output = BTreeMap::new();
    for part_name in std::iter::once(STYLES_PART).chain(SUPPORTING_STYLE_PARTS) {
        match archive.by_name(part_name) {
            Ok(mut part) => {
                let mut contents = Vec::new();
                part.read_to_end(&mut contents)
                    .map_err(|source| Error::ReadTemplate {
                        path: path.to_owned(),
                        source,
                    })?;
                output.insert(part_name.to_owned(), contents);
            }
            Err(error) if part_name == STYLES_PART => {
                return Err(Error::InvalidTemplate {
                    path: path.to_owned(),
                    message: format!("missing {STYLES_PART}: {error}"),
                });
            }
            Err(_) => {}
        }
    }
    Ok(output)
}

fn parse_styles(path: &Path, xml: &[u8]) -> Result<Vec<TemplateStyle>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut current: Option<(String, TemplateStyleKind, Option<String>)> = None;
    let mut defined = Vec::new();
    let mut latent_names = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) if local_name(element.name().as_ref()) == b"style" => {
                let id = attribute(&reader, &element, b"styleId")?.unwrap_or_default();
                let kind = attribute(&reader, &element, b"type")?
                    .as_deref()
                    .map(style_kind)
                    .unwrap_or(TemplateStyleKind::Other);
                current = Some((id, kind, None));
            }
            Ok(Event::Empty(element)) if local_name(element.name().as_ref()) == b"style" => {
                let id = attribute(&reader, &element, b"styleId")?.unwrap_or_default();
                if !id.is_empty() {
                    let kind = attribute(&reader, &element, b"type")?
                        .as_deref()
                        .map(style_kind)
                        .unwrap_or(TemplateStyleKind::Other);
                    defined.push(TemplateStyle {
                        name: id.clone(),
                        id,
                        kind,
                    });
                }
            }
            Ok(Event::Start(element) | Event::Empty(element))
                if local_name(element.name().as_ref()) == b"name" =>
            {
                if let Some((_, _, name)) = &mut current {
                    *name = attribute(&reader, &element, b"val")?;
                }
            }
            Ok(Event::End(element)) if local_name(element.name().as_ref()) == b"style" => {
                if let Some((id, kind, name)) = current.take()
                    && !id.is_empty()
                {
                    defined.push(TemplateStyle {
                        name: name.unwrap_or_else(|| id.clone()),
                        id,
                        kind,
                    });
                }
            }
            Ok(Event::Start(element) | Event::Empty(element))
                if local_name(element.name().as_ref()) == b"lsdException" =>
            {
                if let Some(name) = attribute(&reader, &element, b"name")? {
                    if !name.is_empty() {
                        latent_names.push(name);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(Error::InvalidTemplate {
                    path: path.to_owned(),
                    message: format!("invalid {STYLES_PART}: {error}"),
                });
            }
        }
        buffer.clear();
    }

    Ok(merge_defined_and_latent(defined, &latent_names))
}

fn merge_defined_and_latent(
    defined: Vec<TemplateStyle>,
    latent_names: &[String],
) -> Vec<TemplateStyle> {
    let german = defined.iter().any(|style| {
        style.id.eq_ignore_ascii_case("Standard")
            || style.id.eq_ignore_ascii_case("berschrift1")
            || style.id.eq_ignore_ascii_case("Absatz-Standardschriftart")
    });

    let mut occupied_ids: std::collections::HashSet<String> = defined
        .iter()
        .map(|style| normalize_style_key(&style.id))
        .collect();
    let mut occupied_names: std::collections::HashSet<String> = defined
        .iter()
        .map(|style| normalize_style_key(&style.name))
        .collect();

    let mut styles = defined;
    for latent_name in latent_names {
        let name_key = normalize_style_key(latent_name);
        if occupied_names.contains(&name_key) {
            continue;
        }
        let (id, display_name) = latent_to_id_and_label(latent_name, german);
        let id_key = normalize_style_key(&id);
        if occupied_ids.contains(&id_key) {
            continue;
        }
        occupied_ids.insert(id_key);
        occupied_names.insert(name_key);
        styles.push(TemplateStyle {
            id,
            name: display_name,
            kind: guess_latent_kind(latent_name),
        });
    }
    styles
}

fn normalize_style_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn english_style_id(latent_name: &str) -> String {
    latent_name
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn latent_to_id_and_label(latent_name: &str, german: bool) -> (String, String) {
    if german {
        if let Some((id, label)) = german_latent_mapping(latent_name) {
            return (id.to_owned(), label.to_owned());
        }
    }
    let id = english_style_id(latent_name);
    (id, latent_name.to_owned())
}

/// Bekannte DE-Style-IDs (wie Word sie in deutschen Installationen anlegt).
fn german_latent_mapping(latent_name: &str) -> Option<(&'static str, &'static str)> {
    match latent_name {
        "Normal" => Some(("Standard", "Standard")),
        "heading 1" => Some(("berschrift1", "Überschrift 1")),
        "heading 2" => Some(("berschrift2", "Überschrift 2")),
        "heading 3" => Some(("berschrift3", "Überschrift 3")),
        "heading 4" => Some(("berschrift4", "Überschrift 4")),
        "heading 5" => Some(("berschrift5", "Überschrift 5")),
        "heading 6" => Some(("berschrift6", "Überschrift 6")),
        "heading 7" => Some(("berschrift7", "Überschrift 7")),
        "heading 8" => Some(("berschrift8", "Überschrift 8")),
        "heading 9" => Some(("berschrift9", "Überschrift 9")),
        "Title" => Some(("Titel", "Titel")),
        "Subtitle" => Some(("Untertitel", "Untertitel")),
        "Quote" => Some(("Zitat", "Zitat")),
        "Intense Quote" => Some(("IntensivesZitat", "Intensives Zitat")),
        "List Paragraph" => Some(("Listenabsatz", "Listenabsatz")),
        "No Spacing" => Some(("KeinLeerraum", "Kein Leerraum")),
        "Strong" => Some(("Fett", "Fett")),
        "Emphasis" => Some(("Kursiv", "Kursiv")),
        "Subtle Emphasis" => Some(("SchwacheHervorhebung", "Schwache Hervorhebung")),
        "Intense Emphasis" => Some(("StarkeHervorhebung", "Starke Hervorhebung")),
        "Book Title" => Some(("Buchtitel", "Buchtitel")),
        "Hyperlink" => Some(("Hyperlink", "Hyperlink")),
        "FollowedHyperlink" => Some(("BesuchterHyperlink", "Besuchter Hyperlink")),
        "Default Paragraph Font" => Some(("Absatz-Standardschriftart", "Absatz-Standardschriftart")),
        "Normal Table" => Some(("NormaleTabelle", "Normale Tabelle")),
        "No List" => Some(("KeineListe", "Keine Liste")),
        "caption" => Some(("Beschriftung", "Beschriftung")),
        "TOC Heading" => Some(("Inhaltsverzeichnisberschrift", "Inhaltsverzeichnisüberschrift")),
        "Header" | "header" => Some(("Kopfzeile", "Kopfzeile")),
        "Footer" | "footer" => Some(("Fuzeile", "Fußzeile")),
        "Footnote Text" | "footnote text" => Some(("Fuotnotentext", "Fußnotentext")),
        "Endnote Text" | "endnote text" => Some(("Endnotentext", "Endnotentext")),
        _ => None,
    }
}

fn guess_latent_kind(latent_name: &str) -> TemplateStyleKind {
    let key = normalize_style_key(latent_name);
    if key.contains("table") || key.contains("grid") {
        TemplateStyleKind::Table
    } else if key == "nolist" || key.starts_with("list") && !key.contains("paragraph") {
        // "List", "List Bullet", … sind Nummerierungs-/Listen-Definitionen
        if key.contains("paragraph") {
            TemplateStyleKind::Paragraph
        } else if key == "nolist" {
            TemplateStyleKind::Numbering
        } else {
            TemplateStyleKind::Numbering
        }
    } else if matches!(
        key.as_str(),
        "strong"
            | "emphasis"
            | "subtleemphasis"
            | "intenseemphasis"
            | "booktitle"
            | "hyperlink"
            | "followedhyperlink"
            | "defaultparagraphfont"
            | "strikethrough"
            | "htmlcode"
            | "footnotereference"
            | "endnotereference"
            | "pagenumber"
            | "linenumber"
    ) || key.ends_with("font")
        || key.contains("hyperlink")
        || key.contains("emphasis")
        || key.contains("reference") && !key.contains("heading")
    {
        TemplateStyleKind::Character
    } else {
        TemplateStyleKind::Paragraph
    }
}

fn attribute(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| Error::WriteDocx(error.to_string()))?;
        if local_name(attribute.key.as_ref()) == name {
            return attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                .map(|value| Some(value.into_owned()))
                .map_err(|error| Error::WriteDocx(error.to_string()));
        }
    }
    Ok(None)
}

fn local_name(qualified: &[u8]) -> &[u8] {
    qualified
        .rsplit(|byte| *byte == b':')
        .next()
        .unwrap_or(qualified)
}

fn style_kind(value: &str) -> TemplateStyleKind {
    match value {
        "paragraph" => TemplateStyleKind::Paragraph,
        "character" => TemplateStyleKind::Character,
        "table" => TemplateStyleKind::Table,
        "numbering" => TemplateStyleKind::Numbering,
        _ => TemplateStyleKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_numbering_defs, merge_list_numberings, merge_list_style_numbering};

    #[test]
    fn merges_high_id_list_defs_into_template_numbering() {
        let template = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0"><w:pStyle w:val="Heading1"/><w:lvlText w:val="%1"/></w:lvl>
  </w:abstractNum>
  <w:num w:numId="2"><w:abstractNumId w:val="0"/></w:num>
</w:numbering>"#;
        let generated = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="1"></w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="1"/></w:num>
  <w:abstractNum w:abstractNumId="1000">
    <w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl>
  </w:abstractNum>
  <w:num w:numId="1000"><w:abstractNumId w:val="1000"/></w:num>
</w:numbering>"#;

        let merged = merge_list_numberings(template, generated, 1000).expect("merge");
        let merged = String::from_utf8(merged).expect("utf8");
        assert!(merged.contains("w:abstractNumId=\"0\""));
        assert!(merged.contains("w:numId=\"2\""));
        assert!(merged.contains("w:abstractNumId=\"1000\""));
        assert!(merged.contains("w:numId=\"1000\""));
        assert!(!merged.contains("w:abstractNumId=\"1\""));
        assert!(merged.contains("Heading1"));
        assert!(merged.contains("bullet"));
    }

    #[test]
    fn merges_list_style_num_pr_into_template_styles() {
        let template = concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
            r#"<w:style w:type="paragraph" w:styleId="Aufzhlung">"#,
            r#"<w:name w:val="Aufzaehlung"/>"#,
            r#"<w:pPr><w:spacing w:after="120"/></w:pPr>"#,
            r#"</w:style></w:styles>"#,
        );
        let generated = concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
            r#"<w:style w:type="paragraph" w:styleId="Aufzhlung">"#,
            r#"<w:name w:val="Aufzhlung"/>"#,
            r#"<w:pPr><w:numPr><w:numId w:val="1000"/></w:numPr></w:pPr>"#,
            r#"</w:style></w:styles>"#,
        );

        let merged =
            merge_list_style_numbering(template.as_bytes(), generated.as_bytes()).expect("merge");
        let merged = String::from_utf8(merged).expect("utf8");
        assert!(merged.contains("Aufzaehlung"));
        assert!(merged.contains("w:spacing"));
        assert!(merged.contains("w:val=\"1000\""));
        assert!(merged.contains("<w:numPr>"));
    }

    #[test]
    fn extracts_only_ids_at_or_above_base() {
        let xml = br#"<w:numbering>
  <w:abstractNum w:abstractNumId="2"></w:abstractNum>
  <w:abstractNum w:abstractNumId="1000"></w:abstractNum>
  <w:num w:numId="3"></w:num>
  <w:num w:numId="1001"></w:num>
</w:numbering>"#;
        let extracted = extract_numbering_defs(xml, 1000).expect("extract");
        let text = String::from_utf8(extracted).expect("utf8");
        assert!(text.contains("1000"));
        assert!(text.contains("1001"));
        assert!(!text.contains("abstractNumId=\"2\""));
        assert!(!text.contains("numId=\"3\""));
    }
}
