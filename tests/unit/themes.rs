use std::fs;

use super::{BUNDLED, ThemeSet};
use crate::model::ids::Origin;
use crate::model::theme::Theme;

#[test]
fn every_bundled_theme_parses_and_declares_its_registry_name() {
    assert!(!BUNDLED.is_empty());
    for (name, source) in BUNDLED {
        let theme = Theme::parse(
            std::path::PathBuf::from(format!("themes/{name}.toml")),
            source,
            Origin::Bundled,
        )
        .unwrap()
        .validate()
        .unwrap();
        assert_eq!(theme.id.as_str(), *name);
        assert_eq!(theme.origin, Origin::Bundled);
    }
}

#[test]
fn a_missing_user_directory_loads_only_bundled_themes() {
    let dir = tempfile::tempdir().unwrap();
    let set = ThemeSet::load(&dir.path().join("absent")).unwrap();
    let ids: Vec<String> = set.iter().map(|theme| theme.id.to_string()).collect();
    let expected: Vec<String> = BUNDLED.iter().map(|(name, _)| (*name).to_owned()).collect();
    assert_eq!(ids, expected);
}

#[test]
fn a_user_theme_shadows_the_bundled_theme() {
    let dir = tempfile::tempdir().unwrap();
    let themes = dir.path().join("themes");
    fs::create_dir(&themes).unwrap();
    fs::write(
        themes.join("shadow-nord.toml"),
        include_str!("../fixtures/themes/shadow-nord.toml"),
    )
    .unwrap();

    let set = ThemeSet::load(&themes).unwrap();
    let nord = set.get("nord").unwrap();
    assert_eq!(nord.origin, Origin::User);
    assert_eq!(nord.name, "Shadow Nord");
    assert_eq!(
        nord.data.palette[crate::model::palette::Base16Entry::B00].to_string(),
        "#101820"
    );
    let count = set
        .iter()
        .filter(|theme| theme.id.as_str() == "nord")
        .count();
    assert_eq!(count, 1, "the bundled entry is replaced, not duplicated");
}

#[test]
fn two_user_theme_files_with_one_id_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let themes = dir.path().join("themes");
    fs::create_dir(&themes).unwrap();
    let source = include_str!("../fixtures/themes/shadow-nord.toml");
    fs::write(themes.join("a.toml"), source).unwrap();
    fs::write(themes.join("b.toml"), source).unwrap();

    let error = ThemeSet::load(&themes).unwrap_err();
    assert!(
        matches!(error, crate::Error::DuplicateThemeId { .. }),
        "{error}"
    );
    let message = error.to_string();
    assert!(message.contains("nord"), "{message}");
    assert!(
        message.contains("a.toml") && message.contains("b.toml"),
        "{message}"
    );
}

#[test]
fn non_toml_entries_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let themes = dir.path().join("themes");
    fs::create_dir(&themes).unwrap();
    fs::create_dir(themes.join("nested")).unwrap();
    fs::write(themes.join("notes.txt"), "not a theme").unwrap();

    let set = ThemeSet::load(&themes).unwrap();
    let ids: Vec<&str> = set.iter().map(|theme| theme.id.as_str()).collect();
    assert_eq!(ids, ["nord"], "only the bundled theme remains");
}
