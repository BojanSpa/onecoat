use std::path::PathBuf;

use serde_json::Value;

use super::{ProfileScheme, fragment_edits, scheme, settings_edits, window_theme};
use crate::jsonc::Edit;
use crate::model::ids::{Origin, Slot};
use crate::model::palette::{AnsiSlot, Base16Entry};
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

fn colour(value: &Value) -> String {
    value.as_str().unwrap().to_owned()
}

fn element(edit: &Edit) -> Value {
    match edit {
        Edit::Element { value, .. } => value.clone(),
        other => panic!("expected an element edit, got {other:?}"),
    }
}

#[test]
fn every_scheme_key_carries_its_base16_role() {
    let theme = theme("nord", crate::themes::BUNDLED[0].1);
    let scheme = scheme(&theme).unwrap();
    let palette = &theme.data.palette;
    let ansi = &theme.data.ansi;

    assert_eq!(scheme["name"], "onecoat-dark");
    assert_eq!(
        colour(&scheme["background"]),
        palette[Base16Entry::B00].to_string()
    );
    assert_eq!(
        colour(&scheme["foreground"]),
        palette[Base16Entry::B05].to_string()
    );
    assert_eq!(
        colour(&scheme["cursorColor"]),
        palette[Base16Entry::B0D].to_string()
    );
    assert_eq!(
        colour(&scheme["selectionBackground"]),
        palette[Base16Entry::B02].to_string()
    );
    for (key, slot) in [
        ("black", AnsiSlot::Black),
        ("red", AnsiSlot::Red),
        ("green", AnsiSlot::Green),
        ("yellow", AnsiSlot::Yellow),
        ("blue", AnsiSlot::Blue),
        ("purple", AnsiSlot::Purple),
        ("cyan", AnsiSlot::Cyan),
        ("white", AnsiSlot::White),
        ("brightBlack", AnsiSlot::BrightBlack),
        ("brightRed", AnsiSlot::BrightRed),
        ("brightGreen", AnsiSlot::BrightGreen),
        ("brightYellow", AnsiSlot::BrightYellow),
        ("brightBlue", AnsiSlot::BrightBlue),
        ("brightPurple", AnsiSlot::BrightPurple),
        ("brightCyan", AnsiSlot::BrightCyan),
        ("brightWhite", AnsiSlot::BrightWhite),
    ] {
        assert_eq!(
            colour(&scheme[key]),
            ansi[slot].to_string(),
            "scheme key {key}"
        );
    }
    assert_eq!(
        colour(&scheme["white"]),
        palette[Base16Entry::B06].to_string()
    );
    assert_eq!(
        colour(&scheme["brightWhite"]),
        palette[Base16Entry::B07].to_string()
    );
    assert_eq!(
        colour(&scheme["purple"]),
        palette[Base16Entry::B0E].to_string()
    );
}

#[test]
fn overrides_replace_only_the_roles_they_name() {
    let theme = theme(
        "overrides",
        include_str!("../../fixtures/themes/overrides.toml"),
    );
    let scheme = scheme(&theme).unwrap();

    assert_eq!(colour(&scheme["red"]), "#ff0000");
    assert_eq!(
        colour(&scheme["green"]),
        theme.data.palette[Base16Entry::B0B].to_string(),
        "an ANSI slot the file did not override keeps its derived colour"
    );
    assert_eq!(colour(&scheme["cursorColor"]), "#d8dee9");
    assert_eq!(
        colour(&scheme["background"]),
        "#101820",
        "the background override reaches the scheme"
    );
}

#[test]
fn the_window_theme_takes_the_slot_and_the_surfaces_from_the_palette() {
    let theme = theme("nord", crate::themes::BUNDLED[0].1);
    let palette = &theme.data.palette;
    let window = window_theme(&theme).unwrap();

    assert_eq!(window["name"], "onecoat-dark");
    assert_eq!(window["window"]["applicationTheme"], "dark");
    assert_eq!(
        colour(&window["window"]["frame"]),
        palette[Base16Entry::B00].to_string()
    );
    assert_eq!(
        colour(&window["tabRow"]["background"]),
        palette[Base16Entry::B01].to_string()
    );
    assert_eq!(
        colour(&window["tabRow"]["unfocusedBackground"]),
        palette[Base16Entry::B00].to_string()
    );
    assert_eq!(
        colour(&window["tab"]["background"]),
        palette[Base16Entry::B01].to_string()
    );
    assert_eq!(
        colour(&window["tab"]["unfocusedBackground"]),
        palette[Base16Entry::B00].to_string()
    );
}

#[test]
fn the_window_theme_follows_the_background_override() {
    let theme = theme(
        "overrides",
        include_str!("../../fixtures/themes/overrides.toml"),
    );
    let window = window_theme(&theme).unwrap();

    assert_eq!(colour(&window["window"]["frame"]), "#101820");
    assert_eq!(colour(&window["tabRow"]["unfocusedBackground"]), "#101820");
}

#[test]
fn the_settings_edits_name_the_pair_the_window_theme_and_the_scheme() {
    let theme = theme("nord", crate::themes::BUNDLED[0].1);
    let edits = settings_edits(&theme, ProfileScheme::Report).unwrap();
    assert_eq!(edits.len(), 3);

    assert_eq!(
        edits[0],
        Edit::Pair {
            key: crate::jsonc::Key::parse("theme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: crate::jsonc::Fill::BuiltIn,
        }
    );
    assert_eq!(element(&edits[1])["name"], "onecoat-dark");
    assert_eq!(element(&edits[1])["window"]["applicationTheme"], "dark");
    assert_eq!(
        edits[2],
        Edit::Pair {
            key: crate::jsonc::Key::parse("profiles.defaults.colorScheme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: crate::jsonc::Fill::FromString,
        }
    );
}

#[test]
fn the_fragment_edit_upserts_the_scheme_by_name() {
    let theme = theme("nord", crate::themes::BUNDLED[0].1);
    let edits = fragment_edits(&theme).unwrap();
    assert_eq!(edits.len(), 1);

    match &edits[0] {
        Edit::Element { key, name, value } => {
            assert_eq!(key, &crate::jsonc::Key::parse("schemes"));
            assert_eq!(name, "onecoat-dark");
            assert_eq!(value, &scheme(&theme).unwrap());
        }
        other => panic!("expected an element edit, got {other:?}"),
    }
}

#[test]
fn the_repoint_edit_joins_the_settings_edits_only_under_all() {
    let theme = theme("nord", crate::themes::BUNDLED[0].1);

    let reported = settings_edits(&theme, ProfileScheme::Report).unwrap();
    assert_eq!(reported.len(), 3);

    let all = settings_edits(&theme, ProfileScheme::All).unwrap();
    assert_eq!(all.len(), 4);
    assert_eq!(
        all[3],
        Edit::Repoint {
            key: crate::jsonc::Key::parse("profiles.list"),
            field: "colorScheme".to_owned(),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: crate::jsonc::Fill::FromString,
        }
    );
}
