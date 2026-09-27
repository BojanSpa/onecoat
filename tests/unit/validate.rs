use std::path::PathBuf;

use crate::Error;
use crate::model::color::HexColor;
use crate::model::ids::{Appearance, Origin};
use crate::model::palette::{AnsiSlot, Base16Entry};
use crate::model::theme::{OverrideValue, Theme, Validated};

fn validate_like_themeset(name: &str, source: &str) -> Result<Theme<Validated>, Error> {
    let path = PathBuf::from(format!("tests/fixtures/themes/{name}"));
    Theme::parse(path, source, Origin::User)?.validate()
}

macro_rules! source {
    ($name:literal) => {
        include_str!(concat!("../fixtures/themes/", $name))
    };
}

macro_rules! fixture {
    ($name:literal) => {
        validate_like_themeset($name, source!($name))
    };
}

#[test]
fn incomplete_palette_names_the_missing_entry() {
    let error = fixture!("incomplete-palette.toml").unwrap_err();
    let message = error.to_string();

    assert!(
        matches!(error, Error::PaletteIncomplete { .. }),
        "{message}"
    );

    assert!(
        message.contains("tests/fixtures/themes/incomplete-palette.toml"),
        "{message}"
    );

    assert!(message.contains("base0D"), "{message}");
    assert!(message.contains("[palette]"), "{message}");
}

#[test]
fn malformed_palette_colour_names_the_key_and_value() {
    let error = fixture!("malformed-color.toml").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("base07"), "{message}");
    assert!(message.contains("#xyz"), "{message}");
}

#[test]
fn a_declaration_that_contradicts_the_background_is_rejected() {
    let error = fixture!("light-declared-dark.toml").unwrap_err();
    let message = error.to_string();

    assert!(
        matches!(error, Error::AppearanceMismatch { .. }),
        "{message}"
    );

    assert!(message.contains("appearance = \"light\""), "{message}");
    assert!(message.contains("set appearance = \"dark\""), "{message}");

    let error = fixture!("dark-declared-light.toml").unwrap_err();
    let message = error.to_string();

    assert!(
        matches!(error, Error::AppearanceMismatch { .. }),
        "{message}"
    );

    assert!(message.contains("set appearance = \"light\""), "{message}");
}

#[test]
fn declarations_that_match_the_background_validate() {
    let dark = source!("light-declared-dark.toml")
        .replace("appearance = \"light\"", "appearance = \"dark\"");

    let theme = validate_like_themeset("light-declared-dark.toml", &dark).unwrap();
    assert_eq!(theme.appearance, Appearance::Dark);

    let light = source!("dark-declared-light.toml")
        .replace("appearance = \"dark\"", "appearance = \"light\"");

    let theme = validate_like_themeset("dark-declared-light.toml", &light).unwrap();
    assert_eq!(theme.appearance, Appearance::Light);
}

#[test]
fn reserved_omp_builtin_ids_are_rejected() {
    let error = fixture!("reserved-id.toml").unwrap_err();
    let message = error.to_string();
    assert!(matches!(error, Error::ReservedId { .. }), "{message}");
    assert!(message.contains("titanium"), "{message}");
    assert!(message.contains("rename"), "{message}");
}

#[test]
fn unknown_ansi_key_lists_the_windows_terminal_keys() {
    let error = fixture!("unknown-ansi-key.toml").unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("unknown key `magenta` in [ansi]"),
        "{message}"
    );

    assert!(message.contains("purple"), "{message}");
}

#[test]
fn unknown_root_key_is_rejected() {
    let error = fixture!("unknown-root-key.toml").unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("unknown key `foo` in the theme file"),
        "{message}"
    );
}

#[test]
fn overrides_are_interpreted_into_the_validated_theme() {
    let theme = fixture!("overrides.toml").unwrap();
    let data = &theme.data;

    assert_eq!(
        data.ansi[AnsiSlot::Red],
        HexColor::parse("#ff0000").unwrap()
    );

    assert_eq!(
        data.ansi[AnsiSlot::Green],
        data.palette[Base16Entry::B0B],
        "an ANSI slot the file did not override keeps its derived colour"
    );

    assert_eq!(data.wt.cursor_color, HexColor::parse("#d8dee9"));

    assert_eq!(
        data.omp.get("mdHeading"),
        Some(&OverrideValue::Color(HexColor::parse("#81a1c1").unwrap()))
    );

    assert_eq!(
        data.omp.get("thinkingOff"),
        Some(&OverrideValue::Index(240))
    );

    assert_eq!(
        data.herdr.get("name"),
        Some(&OverrideValue::Text("terminal".to_owned()))
    );
}

#[test]
fn a_second_spelling_of_a_palette_key_is_rejected() {
    let source = source!("overrides.toml").replace(
        "base0A = \"#EBCB8B\"",
        "base0A = \"#EBCB8B\"\nbase0a = \"#000000\"",
    );

    let error = validate_like_themeset("duplicate.toml", &source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("unknown key `base0a` in [palette]"),
        "{message}"
    );

    assert!(message.contains("base0A"), "{message}");
}

#[test]
fn unknown_target_section_lists_the_targets() {
    let source = source!("overrides.toml").replace("[targets.herdr]", "[targets.vscode]");
    let error = validate_like_themeset("unknown-target.toml", &source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("unknown key `vscode` in [targets]"),
        "{message}"
    );

    assert!(
        message.contains("accepted keys are wt, herdr, omp"),
        "{message}"
    );
}

#[test]
fn windows_terminal_overrides_reject_ansi_keys() {
    let source = source!("overrides.toml").replace(
        "cursorColor = \"#D8DEE9\"",
        "cursorColor = \"#D8DEE9\"\nred = \"#ff0000\"",
    );

    let error = validate_like_themeset("wt-key.toml", &source).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("unknown key `red` in [targets.wt]"),
        "{message}"
    );

    assert!(
        message.contains("background, foreground, cursorColor, selectionBackground"),
        "{message}"
    );
}

#[test]
fn out_of_range_token_indices_are_rejected() {
    let source = source!("overrides.toml").replace("thinkingOff = 240", "thinkingOff = 300");
    let error = validate_like_themeset("index.toml", &source).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("targets.omp.thinkingOff"), "{message}");
    assert!(message.contains("300"), "{message}");
}

#[test]
fn malformed_token_colours_are_rejected() {
    let source =
        source!("overrides.toml").replace("mdHeading = \"#81A1C1\"", "mdHeading = \"#81A1\"");

    let error = validate_like_themeset("token-color.toml", &source).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("targets.omp.mdHeading"), "{message}");
    assert!(message.contains("#81A1"), "{message}");
}
