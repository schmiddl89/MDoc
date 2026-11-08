use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::template::{TemplateStyle, TemplateStyleKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StyleProfile {
    pub name: String,
    pub paragraphs: ParagraphStyleMap,
    pub characters: CharacterStyleMap,
}

impl Default for StyleProfile {
    fn default() -> Self {
        Self {
            name: "Word Standard".to_owned(),
            paragraphs: ParagraphStyleMap::default(),
            characters: CharacterStyleMap::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ParagraphStyleMap {
    pub normal: Option<String>,
    pub heading1: Option<String>,
    pub heading2: Option<String>,
    pub heading3: Option<String>,
    pub heading4: Option<String>,
    pub heading5: Option<String>,
    pub heading6: Option<String>,
    pub block_quote: Option<String>,
    pub code_block: Option<String>,
    pub unordered_list: Option<String>,
    pub ordered_list: Option<String>,
    pub table: Option<String>,
}

impl Default for ParagraphStyleMap {
    fn default() -> Self {
        // Deutsche Word-Style-IDs (UI „Überschrift 1“ usw.), damit Einfügen in
        // Word die Formatvorlage in der Galerie aktiv setzt — nicht Heading1.
        Self {
            normal: Some("Standard".to_owned()),
            heading1: Some("berschrift1".to_owned()),
            heading2: Some("berschrift2".to_owned()),
            heading3: Some("berschrift3".to_owned()),
            heading4: Some("berschrift4".to_owned()),
            heading5: Some("berschrift5".to_owned()),
            heading6: Some("berschrift6".to_owned()),
            block_quote: Some("Zitat".to_owned()),
            code_block: Some("KeinLeerraum".to_owned()),
            unordered_list: Some("Listenabsatz".to_owned()),
            ordered_list: Some("Listenabsatz".to_owned()),
            table: Some("Tabellenraster".to_owned()),
        }
    }
}

impl ParagraphStyleMap {
    #[must_use]
    pub fn heading(&self, level: u8) -> Option<&str> {
        match level {
            1 => self.heading1.as_deref(),
            2 => self.heading2.as_deref(),
            3 => self.heading3.as_deref(),
            4 => self.heading4.as_deref(),
            5 => self.heading5.as_deref(),
            6 => self.heading6.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CharacterStyleMap {
    pub strong: Option<String>,
    pub emphasis: Option<String>,
    /// Fett und kursiv zugleich (`***text***` / verschachtelt).
    pub strong_emphasis: Option<String>,
    pub strikethrough: Option<String>,
    pub inline_code: Option<String>,
    pub link: Option<String>,
}

impl Default for CharacterStyleMap {
    fn default() -> Self {
        Self {
            strong: Some("Fett".to_owned()),
            emphasis: Some("Kursiv".to_owned()),
            strong_emphasis: None,
            strikethrough: Some("Durchgestrichen".to_owned()),
            inline_code: None,
            link: Some("Hyperlink".to_owned()),
        }
    }
}

/// Mappt frühere englische Default-IDs auf deutsche Word-IDs (Insert/Galerie).
#[must_use]
pub fn migrate_legacy_english_ids(mut profile: StyleProfile) -> StyleProfile {
    fn remap(value: &mut Option<String>, from: &str, to: &str) {
        if value.as_deref() == Some(from) {
            *value = Some(to.to_owned());
        }
    }
    remap(&mut profile.paragraphs.normal, "Normal", "Standard");
    remap(&mut profile.paragraphs.heading1, "Heading1", "berschrift1");
    remap(&mut profile.paragraphs.heading2, "Heading2", "berschrift2");
    remap(&mut profile.paragraphs.heading3, "Heading3", "berschrift3");
    remap(&mut profile.paragraphs.heading4, "Heading4", "berschrift4");
    remap(&mut profile.paragraphs.heading5, "Heading5", "berschrift5");
    remap(&mut profile.paragraphs.heading6, "Heading6", "berschrift6");
    remap(&mut profile.paragraphs.block_quote, "Quote", "Zitat");
    remap(&mut profile.paragraphs.code_block, "NoSpacing", "KeinLeerraum");
    remap(
        &mut profile.paragraphs.unordered_list,
        "ListParagraph",
        "Listenabsatz",
    );
    remap(
        &mut profile.paragraphs.ordered_list,
        "ListParagraph",
        "Listenabsatz",
    );
    remap(&mut profile.paragraphs.table, "TableGrid", "Tabellenraster");
    remap(&mut profile.characters.strong, "Strong", "Fett");
    remap(&mut profile.characters.emphasis, "Emphasis", "Kursiv");
    remap(
        &mut profile.characters.strikethrough,
        "Strikethrough",
        "Durchgestrichen",
    );
    profile
}

/// Derives a style profile from a Word template by matching DE/EN names and IDs.
pub fn profile_from_template(path: impl AsRef<Path>) -> Result<StyleProfile> {
    let path = path.as_ref();
    let styles = crate::template::inspect(path)?;
    Ok(profile_from_styles(
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Vorlage"),
        &styles,
    ))
}

pub fn profile_from_styles(name: &str, styles: &[TemplateStyle]) -> StyleProfile {
    let mut profile = StyleProfile {
        name: format!("Auto: {name}"),
        ..StyleProfile::default()
    };

    let paragraph_styles: Vec<_> = styles
        .iter()
        .filter(|style| {
            matches!(
                style.kind,
                TemplateStyleKind::Paragraph | TemplateStyleKind::Other
            )
        })
        .collect();
    let character_styles: Vec<_> = styles
        .iter()
        .filter(|style| style.kind == TemplateStyleKind::Character)
        .collect();

    if let Some(id) = find_style(
        &paragraph_styles,
        &["normal", "standard", "bodytext", "textkoerper"],
    ) {
        profile.paragraphs.normal = Some(id);
    }
    for level in 1..=6 {
        let aliases = heading_aliases(level);
        if let Some(id) = find_style(&paragraph_styles, &aliases) {
            match level {
                1 => profile.paragraphs.heading1 = Some(id),
                2 => profile.paragraphs.heading2 = Some(id),
                3 => profile.paragraphs.heading3 = Some(id),
                4 => profile.paragraphs.heading4 = Some(id),
                5 => profile.paragraphs.heading5 = Some(id),
                6 => profile.paragraphs.heading6 = Some(id),
                _ => {}
            }
        }
    }
    if let Some(id) = find_style(
        &paragraph_styles,
        &[
            "quote",
            "zitat",
            "blockzitat",
            "blocktext",
            "intensequote",
            "intensiveszitat",
        ],
    ) {
        profile.paragraphs.block_quote = Some(id);
    }
    if let Some(id) = find_style(
        &paragraph_styles,
        &["nospacing", "keinabstand", "htmlpreformatted", "sourcecode", "code"],
    ) {
        profile.paragraphs.code_block = Some(id);
    }
    if let Some(id) = find_style(
        &paragraph_styles,
        &[
            "listparagraph",
            "listenabsatz",
            "listbullet",
            "listnumber",
            "aufzaehlung",
            "aufzahlung",
        ],
    ) {
        profile.paragraphs.unordered_list = Some(id.clone());
        profile.paragraphs.ordered_list = Some(id);
    }
    if let Some(id) = find_style(&paragraph_styles, &["tablegrid", "tabellenraster", "table"]) {
        profile.paragraphs.table = Some(id);
    }

    if let Some(id) = find_style(
        &character_styles,
        &["strong", "fett", "intenseemphasis", "starkehervorhebung"],
    ) {
        profile.characters.strong = Some(id);
    }
    if let Some(id) = find_style(&character_styles, &["emphasis", "kursiv", "hervorhebung"]) {
        profile.characters.emphasis = Some(id);
    }
    if let Some(id) = find_style(
        &character_styles,
        &[
            "strongemphasis",
            "fettkursiv",
            "bolditalic",
            "booktitle",
            "buchtitel",
        ],
    ) {
        profile.characters.strong_emphasis = Some(id);
    }
    if let Some(id) = find_style(&character_styles, &["strikethrough", "durchgestrichen"]) {
        profile.characters.strikethrough = Some(id);
    }
    if let Some(id) = find_style(
        &character_styles,
        &["htmlcode", "code", "verbatimchar", "quelltext"],
    ) {
        profile.characters.inline_code = Some(id);
    }
    if let Some(id) = find_style(
        &character_styles,
        &["hyperlink", "hyperlinkfolg", "internerhyperlink"],
    ) {
        profile.characters.link = Some(id);
    }

    profile
}

fn heading_aliases(level: u8) -> Vec<String> {
    let n = level.to_string();
    vec![
        format!("heading{n}"),
        format!("ueberschrift{n}"),
        format!("uberschrift{n}"),
        format!("berschrift{n}"),
        format!("titel{n}"),
        format!("title{n}"),
    ]
}

fn find_style<S: AsRef<str>>(styles: &[&TemplateStyle], aliases: &[S]) -> Option<String> {
    let normalized_aliases: Vec<String> = aliases
        .iter()
        .map(|alias| normalize_key(alias.as_ref()))
        .collect();

    for style in styles {
        let candidates = [normalize_key(&style.name), normalize_key(&style.id)];
        if candidates
            .iter()
            .any(|candidate| normalized_aliases.iter().any(|alias| candidate == alias))
        {
            return Some(style.id.clone());
        }
    }

    // Fuzzy only for umlaut-stripped styleIds ("berschrift1"), never cross-level
    // contains matches like heading1 ⊂ heading10.
    for style in styles {
        let candidates = [normalize_key(&style.name), normalize_key(&style.id)];
        for candidate in &candidates {
            for alias in &normalized_aliases {
                if alias.len() >= 5
                    && candidate.len() >= alias.len()
                    && trailing_level_digit(candidate) == trailing_level_digit(alias)
                    && alphabetic_prefix(candidate) == alphabetic_prefix(alias)
                {
                    return Some(style.id.clone());
                }
            }
        }
    }
    None
}

fn trailing_level_digit(value: &str) -> Option<char> {
    value.chars().rev().find(|ch| ch.is_ascii_digit())
}

fn alphabetic_prefix(value: &str) -> String {
    value
        .chars()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .collect()
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(|ch| match ch {
            '\u{00E4}' | '\u{00C4}' => "ae".chars().collect::<Vec<_>>(),
            '\u{00F6}' | '\u{00D6}' => "oe".chars().collect::<Vec<_>>(),
            '\u{00FC}' | '\u{00DC}' => "ue".chars().collect::<Vec<_>>(),
            '\u{00DF}' => "ss".chars().collect::<Vec<_>>(),
            other => other.to_lowercase().collect::<Vec<_>>(),
        })
        .collect()
}

/// `w:name` / NameLocal zu einer Style-ID.
///
/// Deutsche Word-IDs streichen Umlaute (`Aufzhlung`), die UI zeigt sie mit Umlaut
/// (`Aufzählung`). FormattedText matched beim Einfügen den **Namen**, nicht die ID —
/// sonst entsteht eine zweite Vorlage (z. B. „¶ Aufzählung“).
#[must_use]
pub fn style_ui_name(style_id: &str) -> &str {
    match style_id {
        "Aufzhlung" => "Aufzählung",
        "Standard" => "Standard",
        "Normal" => "Normal",
        "Listenabsatz" => "Listenabsatz",
        "Nummerierung" => "Nummerierung",
        "KeinLeerraum" => "Kein Leerraum",
        "Zitat" => "Zitat",
        "IntensivesZitat" => "Intensives Zitat",
        "Tabellenraster" => "Tabellenraster",
        "Fett" => "Fett",
        "Kursiv" => "Kursiv",
        "FettKursiv" => "Fett Kursiv",
        "Durchgestrichen" => "Durchgestrichen",
        "Hyperlink" => "Hyperlink",
        "berschrift1" => "Überschrift 1",
        "berschrift2" => "Überschrift 2",
        "berschrift3" => "Überschrift 3",
        "berschrift4" => "Überschrift 4",
        "berschrift5" => "Überschrift 5",
        "berschrift6" => "Überschrift 6",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{StyleProfile, profile_from_styles};
    use crate::template::{TemplateStyle, TemplateStyleKind};

    #[test]
    fn partial_profiles_keep_default_style_bindings() {
        let profile: StyleProfile =
            serde_json::from_str(r#"{"name":"Firma","paragraphs":{"heading1":"Firmentitel"}}"#)
                .expect("profile should deserialize");

        assert_eq!(profile.name, "Firma");
        assert_eq!(profile.paragraphs.heading(1), Some("Firmentitel"));
        assert_eq!(profile.paragraphs.heading(2), Some("berschrift2"));
        assert_eq!(profile.paragraphs.normal.as_deref(), Some("Standard"));
    }

    #[test]
    fn style_ui_name_restores_german_umlauts() {
        assert_eq!(super::style_ui_name("Aufzhlung"), "Aufzählung");
        assert_eq!(super::style_ui_name("KeinLeerraum"), "Kein Leerraum");
        assert_eq!(super::style_ui_name("FettKursiv"), "Fett Kursiv");
        assert_eq!(super::style_ui_name("MeineListe"), "MeineListe");
    }

    #[test]
    fn maps_german_template_style_names() {
        let styles = vec![
            TemplateStyle {
                id: "berschrift1".into(),
                name: "\u{00DC}berschrift 1".into(),
                kind: TemplateStyleKind::Paragraph,
            },
            TemplateStyle {
                id: "Standard".into(),
                name: "Standard".into(),
                kind: TemplateStyleKind::Paragraph,
            },
            TemplateStyle {
                id: "Zitat".into(),
                name: "Zitat".into(),
                kind: TemplateStyleKind::Paragraph,
            },
            TemplateStyle {
                id: "Listenabsatz".into(),
                name: "Listenabsatz".into(),
                kind: TemplateStyleKind::Paragraph,
            },
        ];
        let profile = profile_from_styles("Firma", &styles);
        assert_eq!(profile.paragraphs.heading1.as_deref(), Some("berschrift1"));
        assert_eq!(profile.paragraphs.normal.as_deref(), Some("Standard"));
        assert_eq!(profile.paragraphs.block_quote.as_deref(), Some("Zitat"));
        assert_eq!(
            profile.paragraphs.unordered_list.as_deref(),
            Some("Listenabsatz")
        );
    }
}
