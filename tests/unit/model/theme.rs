use std::path::PathBuf;

use super::Theme;
use crate::error::Error;
use crate::model::ids::{Appearance, Origin};

const VALID: &str = include_str!("../../fixtures/themes/overrides.toml");

#[test]
fn parse_reads_the_document_shape() {
    let theme = Theme::parse(
        PathBuf::from("tests/fixtures/themes/overrides.toml"),
        VALID,
        Origin::User,
    )
    .unwrap();

    assert_eq!(theme.id.as_str(), "overrides");
    assert_eq!(theme.name, "Overrides");
    assert_eq!(theme.appearance, Appearance::Dark);
    assert!(!theme.derived);
    assert_eq!(theme.origin, Origin::User);
    assert_eq!(theme.data.palette.len(), 16);
    assert_eq!(theme.data.ansi.len(), 1);
    assert_eq!(theme.data.targets.len(), 3);
}

#[test]
fn unparseable_toml_names_the_file() {
    let error = Theme::parse(PathBuf::from("theme.toml"), "id = ", Origin::User).unwrap_err();
    assert!(matches!(error, Error::ThemeUnparseable { .. }));
    assert!(error.to_string().contains("theme.toml"));
}

#[test]
fn unknown_root_key_lists_the_accepted_keys() {
    let error = Theme::parse(PathBuf::from("theme.toml"), "foo = 1", Origin::User).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("unknown key `foo` in the theme file"),
        "{message}"
    );

    assert!(
        message.contains("accepted keys are id, name, appearance, derived, palette, ansi, targets"),
        "{message}"
    );
}

#[test]
fn missing_and_mistyped_keys_name_the_key() {
    let source = VALID.replace("name = \"Overrides\"", "");
    let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();

    assert!(
        matches!(error, Error::MissingField { key: "name", .. }),
        "{error}"
    );

    let source = VALID.replace("name = \"Overrides\"", "name = 42");
    let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("`name` must be a string"), "{message}");
    assert!(message.contains("found an integer"), "{message}");
}

#[test]
fn unknown_appearance_names_the_value() {
    let source = VALID.replace("appearance = \"dark\"", "appearance = \"dim\"");
    let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();
    assert!(error.to_string().contains("\"dim\""), "{error}");
}

#[test]
fn an_invalid_theme_id_reports_the_offending_character() {
    let source = VALID.replace("id = \"overrides\"", "id = \"Overrides\"");
    let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("theme id `Overrides` contains `O`"),
        "{message}"
    );
}
