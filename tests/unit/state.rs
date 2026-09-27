use super::{Problem, State};
use crate::model::ids::{Slot, Target, ThemeId};

fn theme_id(raw: &str) -> ThemeId {
    ThemeId::parse(raw).unwrap()
}

#[test]
fn a_rendered_state_parses_back_to_the_same_assignment() {
    let mut state = State::none();
    state.assign(Slot::Light, theme_id("nord"));
    state.record([Target::Wt, Target::Omp]);

    let rendered = state.render().unwrap();

    assert_eq!(State::parse(&rendered).unwrap(), state);
}

#[test]
fn a_state_with_no_assignment_renders_null_slots() {
    let rendered = State::none().render().unwrap();

    assert_eq!(
        rendered,
        "{\n  \"slots\": {\n    \"dark\": null,\n    \"light\": null\n  },\n  \"targets\": []\n}\n"
    );
}

#[test]
fn a_document_without_a_slot_key_reads_as_unassigned() {
    let state = State::parse(r#"{"slots": {"dark": null}, "targets": []}"#).unwrap();

    assert_eq!(state.assigned(Slot::Dark), None);
    assert_eq!(state.assigned(Slot::Light), None);
    assert!(state.targets().is_empty());
}

#[test]
fn an_unknown_key_is_reported_with_its_name() {
    let problem = State::parse(r#"{"slots": {}, "targets": [], "verify": {}}"#).unwrap_err();

    assert!(
        matches!(&problem, Problem::Unknown { key } if key == "verify"),
        "{problem}"
    );
}

#[test]
fn an_unknown_key_inside_slots_is_reported_with_its_full_name() {
    let problem = State::parse(r#"{"slots": {"dim": "nord"}, "targets": []}"#).unwrap_err();

    assert!(
        matches!(&problem, Problem::Unknown { key } if key == "slots.dim"),
        "{problem}"
    );
}

#[test]
fn an_invalid_theme_id_names_the_key_that_holds_it() {
    let problem = State::parse(r#"{"slots": {"dark": "Nord!"}, "targets": []}"#).unwrap_err();

    assert!(
        matches!(problem, Problem::Id { key, .. } if key == "slots.dark"),
        "{problem}"
    );
}

#[test]
fn an_unknown_target_is_reported_with_its_name() {
    let problem = State::parse(r#"{"slots": {}, "targets": ["iterm"]}"#).unwrap_err();

    assert!(
        matches!(&problem, Problem::Target { name } if name == "iterm"),
        "{problem}"
    );
}

#[test]
fn a_malformed_document_is_a_document_problem() {
    let problem = State::parse("{\"slots\": ").unwrap_err();

    assert!(matches!(problem, Problem::Document(_)), "{problem}");
}

#[test]
fn assigning_one_slot_keeps_the_other() {
    let mut state = State::none();
    state.assign(Slot::Light, theme_id("nord"));
    state.assign(Slot::Dark, theme_id("one-half"));

    assert_eq!(state.assigned(Slot::Light), Some(&theme_id("nord")));
    assert_eq!(state.assigned(Slot::Dark), Some(&theme_id("one-half")));
}

#[test]
fn recording_targets_replaces_the_previous_list() {
    let mut state = State::none();
    state.record([Target::Wt, Target::Omp]);
    state.record([Target::Herdr]);

    assert_eq!(state.targets(), [Target::Herdr]);
}
