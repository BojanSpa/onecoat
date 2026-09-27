use super::{IdProblem, Slot, ThemeId};

#[test]
fn theme_id_accepts_lowercase_ids() {
    assert_eq!(ThemeId::parse("nord").unwrap().as_str(), "nord");
    assert_eq!(ThemeId::parse("nord-2").unwrap().as_str(), "nord-2");
    assert_eq!(ThemeId::parse("2-tone").unwrap().as_str(), "2-tone");
    assert_eq!(ThemeId::parse("nord-").unwrap().as_str(), "nord-");
}

#[test]
fn theme_id_rejects_an_empty_id() {
    assert_eq!(ThemeId::parse(""), Err(IdProblem::Empty));
}

#[test]
fn theme_id_rejects_characters_outside_lowercase_ascii() {
    assert_eq!(ThemeId::parse("NORD"), Err(IdProblem::BadChar('N')));
    assert_eq!(ThemeId::parse("nord-å"), Err(IdProblem::BadChar('å')));
}

#[test]
fn theme_id_rejects_a_leading_hyphen() {
    assert_eq!(ThemeId::parse("-nord"), Err(IdProblem::BadChar('-')));
}

#[test]
fn appearance_names_its_slot() {
    assert_eq!(Slot::from(super::Appearance::Dark).name(), "dark");
    assert_eq!(Slot::from(super::Appearance::Light).name(), "light");
}
