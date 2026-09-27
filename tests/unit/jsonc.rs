use std::path::Path;

use serde_json::json;

use super::{Edit, Fill, Key, NEW_DOCUMENT, Pin, Splice, pinned, splice, verify};
use crate::Error;
use crate::model::ids::Slot;

fn key(dotted: &str) -> Key {
    Key::parse(dotted)
}

fn splice_ok(source: &str, edits: &[Edit]) -> Splice {
    splice(Path::new("settings.json"), source, edits).unwrap()
}

#[test]
fn a_missing_key_is_inserted_after_the_last_one() {
    let source = "{\n    \"a\": 1\n}";
    let spliced = splice_ok(
        source,
        &[Edit::Set {
            key: key("theme"),
            value: json!({ "dark": "onecoat-dark" }),
        }],
    );

    assert_eq!(
        spliced.text,
        "{\n    \"a\": 1,\n    \"theme\": {\n        \"dark\": \"onecoat-dark\"\n    }\n}"
    );
    assert_eq!(spliced.changed, [key("theme")]);
}

#[test]
fn missing_intermediates_are_created() {
    let spliced = splice_ok(
        "{}\n",
        &[Edit::Pair {
            key: key("profiles.defaults.colorScheme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::FromString,
        }],
    );

    assert_eq!(
        spliced.text,
        "{\n  \"profiles\": {\n    \"defaults\": {\n      \"colorScheme\": {\n        \"dark\": \"onecoat-dark\"\n      }\n    }\n  }\n}\n"
    );
    assert_eq!(spliced.changed, [key("profiles.defaults.colorScheme")]);
}

#[test]
fn a_pair_side_leaves_the_other_appearance_alone() {
    let source = "{\"theme\": {\"light\": \"light\", \"dark\": \"dark\"}}";
    let spliced = splice_ok(
        source,
        &[Edit::Pair {
            key: key("theme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::BuiltIn,
        }],
    );

    assert_eq!(
        spliced.text,
        "{\"theme\": {\"light\": \"light\", \"dark\": \"onecoat-dark\"}}"
    );
    assert_eq!(spliced.changed, [key("theme")]);
}

#[test]
fn a_pair_carries_a_bare_string_into_the_other_side_when_asked() {
    let with_string = "{\n  \"theme\": \"dark\"\n}\n";
    let carried = splice_ok(
        with_string,
        &[Edit::Pair {
            key: key("theme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::FromString,
        }],
    );
    assert_eq!(
        carried.text,
        "{\n  \"theme\": {\n    \"dark\": \"onecoat-dark\",\n    \"light\": \"dark\"\n  }\n}\n"
    );

    let dropped = splice_ok(
        with_string,
        &[Edit::Pair {
            key: key("theme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::BuiltIn,
        }],
    );
    assert_eq!(
        dropped.text,
        "{\n  \"theme\": {\n    \"dark\": \"onecoat-dark\"\n  }\n}\n"
    );
}

#[test]
fn an_array_element_is_replaced_in_place_and_foreign_elements_survive() {
    let source = "{\n  \"themes\": [\n    {\n      \"name\": \"mine\",\n      \"frame\": \"#000000\"\n    },\n    {\n      \"name\": \"onecoat-dark\",\n      \"frame\": \"#111111\"\n    }\n  ]\n}\n";
    let spliced = splice_ok(
        source,
        &[Edit::Element {
            key: key("themes"),
            name: "onecoat-dark".to_owned(),
            value: json!({ "name": "onecoat-dark", "frame": "#0b1018" }),
        }],
    );

    assert_eq!(
        spliced.text,
        "{\n  \"themes\": [\n    {\n      \"name\": \"mine\",\n      \"frame\": \"#000000\"\n    },\n    {\n      \"frame\": \"#0b1018\",\n      \"name\": \"onecoat-dark\"\n    }\n  ]\n}\n"
    );
    assert_eq!(spliced.changed, [key("themes")]);
}

#[test]
fn an_array_element_is_appended_when_its_name_is_absent() {
    let spliced = splice_ok(
        "{\n  \"themes\": [\n    {\n      \"name\": \"mine\"\n    }\n  ]\n}\n",
        &[Edit::Element {
            key: key("themes"),
            name: "onecoat-light".to_owned(),
            value: json!({ "name": "onecoat-light" }),
        }],
    );

    assert_eq!(
        spliced.text,
        "{\n  \"themes\": [\n    {\n      \"name\": \"mine\"\n    },\n    {\n      \"name\": \"onecoat-light\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn an_absent_array_holds_the_element_alone() {
    let spliced = splice_ok(
        "{}\n",
        &[Edit::Element {
            key: key("schemes"),
            name: "onecoat-dark".to_owned(),
            value: json!({ "name": "onecoat-dark" }),
        }],
    );

    assert_eq!(
        spliced.text,
        "{\n  \"schemes\": [\n    {\n      \"name\": \"onecoat-dark\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn a_created_document_keeps_the_seed_around_the_new_key() {
    let spliced = splice_ok(
        NEW_DOCUMENT,
        &[Edit::Element {
            key: key("schemes"),
            name: "onecoat-dark".to_owned(),
            value: json!({ "name": "onecoat-dark" }),
        }],
    );

    assert_eq!(
        spliced.text,
        "{\n  \"schemes\": [\n    {\n      \"name\": \"onecoat-dark\"\n    }\n  ]\n}\n"
    );
}

#[test]
fn comments_line_endings_and_indentation_survive() {
    let source = "{\r\n  // the theme\r\n  \"a\": 1,\r\n  \"theme\": \"dark\"\r\n}";
    let spliced = splice_ok(
        source,
        &[Edit::Pair {
            key: key("theme"),
            side: Slot::Light,
            name: "onecoat-light".to_owned(),
            fill: Fill::FromString,
        }],
    );

    assert_eq!(
        spliced.text,
        "{\r\n  // the theme\r\n  \"a\": 1,\r\n  \"theme\": {\r\n    \"light\": \"onecoat-light\",\r\n    \"dark\": \"dark\"\r\n  }\r\n}"
    );
}

#[test]
fn a_second_identical_splice_changes_nothing() {
    let edits = [
        Edit::Pair {
            key: key("theme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::FromString,
        },
        Edit::Element {
            key: key("themes"),
            name: "onecoat-dark".to_owned(),
            value: json!({ "name": "onecoat-dark" }),
        },
    ];
    let once = splice_ok("{\n  \"themes\": []\n}\n", &edits);
    let twice = splice_ok(&once.text, &edits);

    assert_eq!(twice.text, once.text);
    assert!(twice.changed.is_empty(), "{:?}", twice.changed);
}

#[test]
fn a_key_that_is_not_an_object_is_an_error_rather_than_a_clobber() {
    let error = splice(
        Path::new("settings.json"),
        "{\"profiles\": \"none\"}",
        &[Edit::Pair {
            key: key("profiles.defaults.colorScheme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::FromString,
        }],
    )
    .unwrap_err();

    assert!(
        matches!(&error, Error::KeyNotLocatable { key, .. } if key.to_string() == "profiles"),
        "{error}"
    );
    assert!(error.to_string().contains("profiles"), "{error}");
}

#[test]
fn a_non_object_document_is_an_error() {
    for source in ["[]", "\"text\"", ""] {
        let error = splice(
            Path::new("settings.json"),
            source,
            &[Edit::Set {
                key: key("theme"),
                value: json!("dark"),
            }],
        )
        .unwrap_err();
        assert!(
            matches!(error, Error::KeyNotLocatable { .. }),
            "{source:?} gave {error}"
        );
    }
}

#[test]
fn a_wrongly_typed_key_is_an_error() {
    let array = splice(
        Path::new("settings.json"),
        "{\"themes\": {}}",
        &[Edit::Element {
            key: key("themes"),
            name: "onecoat-dark".to_owned(),
            value: json!({ "name": "onecoat-dark" }),
        }],
    )
    .unwrap_err();
    assert!(
        matches!(&array, Error::KeyNotLocatable { key, expected, .. } if key.to_string() == "themes" && *expected == "a JSON array"),
        "{array}"
    );

    let number = splice(
        Path::new("settings.json"),
        "{\"theme\": 42}",
        &[Edit::Pair {
            key: key("theme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::FromString,
        }],
    )
    .unwrap_err();
    assert!(
        matches!(&number, Error::KeyNotLocatable { expected, .. } if *expected == "a JSON object or a string"),
        "{number}"
    );
}

#[test]
fn a_broken_document_names_its_line_and_column() {
    let error = splice(
        Path::new("settings.json"),
        "{\n    \"a\": 1,\n    \"b\":\n}",
        &[Edit::Set {
            key: key("theme"),
            value: json!("dark"),
        }],
    )
    .unwrap_err();

    assert!(
        matches!(
            error,
            Error::JsoncUnparseable {
                line: 4,
                column: 1,
                ..
            }
        ),
        "{error}"
    );
    assert!(error.to_string().contains("settings.json"), "{error}");
}

#[test]
fn verify_accepts_valid_jsonc_and_reports_the_broken_line() {
    assert!(verify(Path::new("settings.json"), "{\n  // fine\n  \"a\": 1,\n}\n").is_ok());
    let error = verify(Path::new("settings.json"), "{\"a\": }").unwrap_err();
    assert!(
        matches!(error, Error::JsoncUnparseable { line: 1, .. }),
        "{error}"
    );
}

#[test]
fn another_side_of_a_pair_keeps_the_side_it_does_not_own() {
    let source = "{\n  \"theme\": {\n    \"light\": \"light\"\n  }\n}\n";
    let spliced = splice_ok(
        source,
        &[Edit::Pair {
            key: key("theme"),
            side: Slot::Dark,
            name: "onecoat-dark".to_owned(),
            fill: Fill::BuiltIn,
        }],
    );

    assert_eq!(
        spliced.text,
        "{\n  \"theme\": {\n    \"light\": \"light\",\n    \"dark\": \"onecoat-dark\"\n  }\n}\n"
    );
}

fn pins(source: &str) -> Vec<Pin> {
    pinned(
        Path::new("settings.json"),
        source,
        &key("profiles.list"),
        "colorScheme",
    )
    .unwrap()
}

fn repoint(fill: Fill) -> Edit {
    Edit::Repoint {
        key: key("profiles.list"),
        field: "colorScheme".to_owned(),
        side: Slot::Dark,
        name: "onecoat-dark".to_owned(),
        fill,
    }
}

const PINNED: &str = "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"name\": \"Command Prompt\",\n        \"colorScheme\": \"One Half Dark\"\n      },\n      {\n        \"name\": \"Ubuntu\"\n      }\n    ]\n  }\n}\n";

#[test]
fn a_repoint_moves_only_the_elements_that_pin_a_scheme() {
    let spliced = splice_ok(PINNED, &[repoint(Fill::FromString)]);

    assert_eq!(
        spliced.text,
        "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"name\": \"Command Prompt\",\n        \"colorScheme\": {\n          \"dark\": \"onecoat-dark\",\n          \"light\": \"One Half Dark\"\n        }\n      },\n      {\n        \"name\": \"Ubuntu\"\n      }\n    ]\n  }\n}\n"
    );
    assert_eq!(spliced.changed, [key("profiles.list")]);
}

#[test]
fn a_repoint_leaves_the_other_side_of_an_existing_pair_alone() {
    let source = "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"name\": \"PowerShell\",\n        \"colorScheme\": {\n          \"dark\": \"mine\",\n          \"light\": \"other\"\n        }\n      }\n    ]\n  }\n}\n";
    let spliced = splice_ok(source, &[repoint(Fill::FromString)]);

    assert_eq!(
        spliced.text,
        "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"name\": \"PowerShell\",\n        \"colorScheme\": {\n          \"dark\": \"onecoat-dark\",\n          \"light\": \"other\"\n        }\n      }\n    ]\n  }\n}\n"
    );
}

#[test]
fn a_repoint_drops_the_other_side_when_the_fill_is_built_in() {
    let spliced = splice_ok(PINNED, &[repoint(Fill::BuiltIn)]);

    assert_eq!(
        spliced.text,
        "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"name\": \"Command Prompt\",\n        \"colorScheme\": {\n          \"dark\": \"onecoat-dark\"\n        }\n      },\n      {\n        \"name\": \"Ubuntu\"\n      }\n    ]\n  }\n}\n"
    );
}

#[test]
fn a_repoint_changes_nothing_when_no_element_pins_a_scheme() {
    let source = "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"name\": \"Ubuntu\"\n      }\n    ]\n  }\n}\n";
    let spliced = splice_ok(source, &[repoint(Fill::FromString)]);

    assert_eq!(spliced.text, source);
    assert_eq!(spliced.changed, []);
}

#[test]
fn a_repoint_is_a_no_op_when_the_path_is_absent() {
    let spliced = splice_ok("{}\n", &[repoint(Fill::FromString)]);

    assert_eq!(spliced.text, "{}\n");
    assert_eq!(spliced.changed, []);
}

#[test]
fn a_repoint_of_a_value_that_is_not_a_pair_is_an_error() {
    let source = "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"colorScheme\": 7\n      }\n    ]\n  }\n}\n";
    let error = splice(
        Path::new("settings.json"),
        source,
        &[repoint(Fill::FromString)],
    )
    .unwrap_err();

    assert!(matches!(error, Error::KeyNotLocatable { .. }), "{error}");
}

#[test]
fn pins_lists_every_element_that_carries_the_field() {
    let source = "{\n  \"profiles\": {\n    \"list\": [\n      {\n        \"name\": \"Command Prompt\",\n        \"colorScheme\": \"One Half Dark\"\n      },\n      {\n        \"name\": \"Ubuntu\"\n      },\n      {\n        \"guid\": \"{574e775e}\",\n        \"colorScheme\": {\n          \"dark\": \"onecoat-dark\",\n          \"light\": \"onecoat-light\"\n        }\n      }\n    ]\n  }\n}\n";

    assert_eq!(
        pins(source),
        [
            Pin {
                name: "Command Prompt".to_owned(),
                scheme: "One Half Dark".to_owned()
            },
            Pin {
                name: "{574e775e}".to_owned(),
                scheme: "onecoat-dark/onecoat-light".to_owned()
            },
        ]
    );
}

#[test]
fn pins_is_empty_when_the_path_is_absent() {
    assert_eq!(pins("{}\n"), []);
}
