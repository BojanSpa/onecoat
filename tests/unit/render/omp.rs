use std::path::PathBuf;

use serde_json::Value;

use super::{OmpToken, config_edits, splice_config, theme_file};
use crate::Error;
use crate::jsonc::Key;
use crate::model::ids::{Origin, Slot};
use crate::model::palette::Base16Entry;
use crate::model::theme::{Theme, Validated};

const TOKENS: &str = include_str!("../../fixtures/omp/theme-tokens.json");
const CONFIG: &str = include_str!("../../fixtures/omp/config.yml");
const CONFIG_SPLICED: &str = include_str!("../../fixtures/omp/config-spliced.yml");
const CONFIG_LIGHT: &str = include_str!("../../fixtures/omp/config-light.yml");
const DARK_THEME: &str = include_str!("../../fixtures/omp/onecoat-dark.json");
const OMP_OVERRIDES: &str = include_str!("../../fixtures/themes/omp-overrides.toml");
const UNKNOWN_TOKEN: &str = include_str!("../../fixtures/themes/unknown-omp-token.toml");
const TEXT_OVERRIDE: &str = include_str!("../../fixtures/themes/text-omp-override.toml");

fn theme(name: &str, source: &str) -> Theme<Validated> {
    Theme::parse(
        PathBuf::from(format!("themes/{name}.toml")),
        source,
        Origin::Bundled,
    )
    .unwrap()
    .validate()
    .unwrap()
}

fn nord() -> Theme<Validated> {
    theme("nord", crate::themes::BUNDLED[0].1)
}

fn colours(theme: &Theme<Validated>, slot: Slot) -> Value {
    serde_json::from_str::<Value>(&theme_file(theme, slot).unwrap()).unwrap()["colors"].clone()
}

#[test]
fn the_token_enum_matches_the_vendored_list() {
    let vendored: Vec<String> = serde_json::from_str(TOKENS).unwrap();

    let known: Vec<String> = OmpToken::ALL
        .into_iter()
        .map(|token| token.name().to_owned())
        .collect();

    assert_eq!(known, vendored);

    for token in OmpToken::ALL {
        assert_eq!(OmpToken::from_name(token.name()), Some(token));
    }
}

#[test]
fn every_token_is_rendered_with_its_derived_colour() {
    let theme = nord();
    let palette = &theme.data.palette;
    let document: Value = serde_json::from_str(&theme_file(&theme, Slot::Dark).unwrap()).unwrap();

    assert_eq!(document["name"], "onecoat-dark");
    assert_eq!(document["colors"].as_object().unwrap().len(), 69);

    let colors = &document["colors"];
    for (token, entry) in [
        ("text", Base16Entry::B05),
        ("muted", Base16Entry::B04),
        ("dim", Base16Entry::B03),
        ("accent", Base16Entry::B0D),
        ("border", Base16Entry::B02),
        ("borderMuted", Base16Entry::B01),
        ("customMessageBg", Base16Entry::B02),
        ("customMessageLabel", Base16Entry::B0D),
        ("customMessageText", Base16Entry::B05),
        ("link", Base16Entry::B0D),
        ("mdCodeBlockBorder", Base16Entry::B01),
        ("mdQuoteBorder", Base16Entry::B03),
        ("bashMode", Base16Entry::B0C),
        ("pythonMode", Base16Entry::B0D),
        ("thinkingText", Base16Entry::B04),
        ("toolText", Base16Entry::B05),
        ("toolTitle", Base16Entry::B06),
        ("userMessageText", Base16Entry::B06),
        ("toolPendingBg", Base16Entry::B00),
        ("statusLineCost", Base16Entry::B0E),
        ("syntaxNumber", Base16Entry::B09),
        ("error", Base16Entry::B08),
        ("warning", Base16Entry::B0A),
    ] {
        assert_eq!(
            colors[token].as_str().unwrap(),
            palette[entry].to_string(),
            "{token}"
        );
    }
}

#[test]
fn the_generated_file_matches_the_golden() {
    assert_eq!(theme_file(&nord(), Slot::Dark).unwrap(), DARK_THEME);
}

#[test]
fn the_slot_decides_the_name_only() {
    let theme = nord();

    let light: Value = serde_json::from_str(&theme_file(&theme, Slot::Light).unwrap()).unwrap();

    assert_eq!(light["name"], "onecoat-light");
    assert_eq!(light["colors"], colours(&theme, Slot::Dark));
}

#[test]
fn overrides_replace_single_tokens() {
    let theme = theme("omp-overrides", OMP_OVERRIDES);
    let colors = colours(&theme, Slot::Dark);
    let palette = &theme.data.palette;

    assert_eq!(colors["mdHeading"], "#ff00ff");
    assert_eq!(colors["thinkingOff"], "#585858");
    assert_eq!(colors["mdHr"], "#000000");
    assert_eq!(colors["link"], "#ff0000");

    assert_eq!(
        colors["text"],
        palette[Base16Entry::B05].to_string(),
        "a token the file did not override keeps its derived colour"
    );
}

#[test]
fn an_unknown_token_is_rejected() {
    let theme = theme("unknown-omp-token", UNKNOWN_TOKEN);

    let error = theme_file(&theme, Slot::Dark).unwrap_err();

    assert!(
        matches!(&error, Error::UnknownKey { key, section, .. } if key == "bogusToken" && *section == "[targets.omp]"),
        "{error}"
    );
}

#[test]
fn a_text_override_is_rejected() {
    let theme = theme("text-omp-override", TEXT_OVERRIDE);

    let error = theme_file(&theme, Slot::Dark).unwrap_err();

    assert!(
        matches!(&error, Error::MalformedColor { key, value, .. } if key == "targets.omp.text" && value == "plain"),
        "{error}"
    );
}

#[test]
fn a_pin_is_set_and_every_other_byte_is_kept() {
    let spliced = splice_config(
        PathBuf::from("config.yml").as_path(),
        CONFIG,
        &config_edits(Slot::Dark),
    )
    .unwrap();

    assert_eq!(spliced.text, CONFIG_SPLICED);
    assert_eq!(spliced.changed, [Key::parse("theme.dark")]);
}

#[test]
fn the_other_slot_is_pinned_under_the_existing_key() {
    let spliced = splice_config(
        PathBuf::from("config.yml").as_path(),
        CONFIG,
        &config_edits(Slot::Light),
    )
    .unwrap();

    assert_eq!(spliced.text, CONFIG_LIGHT);
    assert_eq!(spliced.changed, [Key::parse("theme.light")]);
}

#[test]
fn splicing_the_same_pin_again_changes_nothing() {
    let path = PathBuf::from("config.yml");

    let spliced = splice_config(path.as_path(), CONFIG_SPLICED, &config_edits(Slot::Dark)).unwrap();

    assert_eq!(spliced.text, CONFIG_SPLICED);
    assert!(spliced.changed.is_empty());
}

#[test]
fn a_file_without_a_theme_block_gains_one() {
    let path = PathBuf::from("config.yml");

    let spliced = splice_config(path.as_path(), "model: x", &config_edits(Slot::Dark)).unwrap();

    assert_eq!(spliced.text, "model: x\ntheme:\n  dark: onecoat-dark\n");
    assert_eq!(spliced.changed, [Key::parse("theme.dark")]);
}

#[test]
fn a_theme_that_is_not_a_mapping_is_refused() {
    let path = PathBuf::from("config.yml");
    for source in ["theme: dainty-nord-1-0\n", "theme: {dark: x}\n"] {
        let error = splice_config(path.as_path(), source, &config_edits(Slot::Dark)).unwrap_err();

        assert!(
            matches!(&error, Error::FieldType { key, .. } if *key == "theme"),
            "{error}"
        );
    }
}

#[test]
fn a_quoted_value_and_its_comment_survive_the_pin() {
    let path = PathBuf::from("config.yml");
    let source = "theme:\r\n  dark: \"old-name\"  # keep me\r\n";

    let spliced = splice_config(path.as_path(), source, &config_edits(Slot::Dark)).unwrap();

    assert_eq!(
        spliced.text,
        "theme:\r\n  dark: onecoat-dark  # keep me\r\n"
    );
}

#[test]
fn a_pin_is_never_written_twice() {
    let path = PathBuf::from("config.yml");
    let source = "theme:\n  dark: onecoat-dark  # keep me\n";

    let spliced = splice_config(path.as_path(), source, &config_edits(Slot::Dark)).unwrap();

    assert_eq!(spliced.text, source);
    assert!(spliced.changed.is_empty());
}
