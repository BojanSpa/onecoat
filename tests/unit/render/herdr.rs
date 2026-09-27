use std::path::PathBuf;

use super::{Edit, HerdrToken, Scope, Value, edits, splice};
use crate::model::color::HexColor;
use crate::model::ids::{Origin, Slot};
use crate::model::palette::Base16Entry;
use crate::model::theme::{Theme, Validated};

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

fn value(edits: &[Edit], path: &str) -> Value {
    edits
        .iter()
        .find(|edit| edit.path.join(".") == path)
        .unwrap_or_else(|| panic!("no edit for {path}"))
        .value
        .clone()
}

#[test]
fn every_token_carries_its_base16_role() {
    let theme = nord();
    let edits = edits(&theme, Slot::Dark).unwrap();
    let palette = &theme.data.palette;
    for token in HerdrToken::ALL {
        let path = match token.scope() {
            Scope::Shared => format!("theme.custom.{}", token.name()),
            Scope::Appearance => format!("theme.custom.dark.{}", token.name()),
        };

        assert_eq!(
            value(&edits, &path),
            Value::Color(palette[token.role()]),
            "token {}",
            token.name()
        );
    }
}

#[test]
fn the_appearance_surfaces_go_in_the_slot_layer_and_the_accents_are_shared() {
    let theme = nord();
    let dark = edits(&theme, Slot::Dark).unwrap();
    let light = edits(&theme, Slot::Light).unwrap();

    assert_eq!(
        value(&dark, "theme.custom.dark.panel_bg"),
        Value::Color(theme.data.palette[Base16Entry::B00])
    );

    assert_eq!(
        value(&light, "theme.custom.light.panel_bg"),
        Value::Color(theme.data.palette[Base16Entry::B00])
    );

    assert!(
        light
            .iter()
            .all(|edit| !edit.path.join(".").starts_with("theme.custom.dark")),
        "the light apply must not write the dark layer"
    );
}

#[test]
fn the_name_defaults_to_terminal_and_an_override_wins() {
    let default = edits(&nord(), Slot::Dark).unwrap();

    assert_eq!(
        value(&default, "theme.name"),
        Value::Text("terminal".to_owned())
    );

    assert_eq!(
        value(&default, "theme.dark_name"),
        Value::Text("terminal".to_owned())
    );

    assert_eq!(
        value(&default, "theme.light_name"),
        Value::Text("terminal".to_owned())
    );

    assert_eq!(value(&default, "theme.auto_switch"), Value::Bool(true));

    let overridden = theme(
        "herdr-overrides",
        include_str!("../../fixtures/themes/herdr-overrides.toml"),
    );

    let edits = edits(&overridden, Slot::Dark).unwrap();

    assert_eq!(value(&edits, "theme.name"), Value::Text("nord".to_owned()));

    assert_eq!(
        value(&edits, "theme.custom.dark.panel_bg"),
        Value::Color(HexColor::from_rgb(0x10, 0x18, 0x20))
    );
}

#[test]
fn a_crlf_document_keeps_its_line_endings() {
    let theme = nord();
    let source = include_str!("../../fixtures/herdr/config-crlf.toml");

    let spliced = splice(
        &PathBuf::from("herdr/config.toml"),
        source,
        &edits(&theme, Slot::Dark).unwrap(),
    )
    .unwrap();

    assert!(spliced.text.contains("\r\n"), "the rewrite kept no CRLF");

    assert!(
        !spliced.text.replace("\r\n", "").contains('\n'),
        "the rewrite introduced a bare newline"
    );

    assert!(spliced.text.contains("[theme.custom.dark]"));
}

#[test]
fn an_unknown_herdr_key_is_refused() {
    let theme = theme(
        "unknown-herdr-token",
        include_str!("../../fixtures/themes/unknown-herdr-token.toml"),
    );

    let error = edits(&theme, Slot::Dark).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("bogusToken"), "{message}");
    assert!(message.contains("[targets.herdr]"), "{message}");
}

#[test]
fn splicing_keeps_comments_foreign_keys_and_adds_the_layer() {
    let theme = nord();
    let source = include_str!("../../fixtures/herdr/config.toml");

    let spliced = splice(
        &PathBuf::from("herdr/config.toml"),
        source,
        &edits(&theme, Slot::Dark).unwrap(),
    )
    .unwrap();

    assert_eq!(
        spliced.text,
        include_str!("../../fixtures/herdr/config-spliced.toml")
    );
}

#[test]
fn splicing_twice_changes_nothing() {
    let theme = nord();
    let source = include_str!("../../fixtures/herdr/config-spliced.toml");

    let spliced = splice(
        &PathBuf::from("herdr/config.toml"),
        source,
        &edits(&theme, Slot::Dark).unwrap(),
    )
    .unwrap();

    assert_eq!(spliced.text, source);
    assert!(spliced.changed.is_empty());
}

#[test]
fn a_document_that_does_not_parse_is_refused() {
    let error = splice(&PathBuf::from("herdr/config.toml"), "[theme\nname = 1", &[]).unwrap_err();

    assert!(error.to_string().contains("not valid TOML"), "{error}");
}
