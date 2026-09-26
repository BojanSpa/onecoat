use std::path::PathBuf;

use serde_json::{Value, json};

fn schema() -> Value {
    serde_json::from_slice(&fixture("profiles.schema.json")).unwrap()
}

fn fragment() -> Value {
    serde_json::from_slice(&fixture("nord-dark-fragment.json")).unwrap()
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("wt")
        .join(name);
    std::fs::read(path).unwrap()
}

fn settings_with(schemes: &Value) -> Value {
    json!({
        "profiles": [],
        "schemes": schemes,
        "defaultProfile": "onecoat",
    })
}

fn validator() -> jsonschema::Validator {
    jsonschema::options().offline().build(&schema()).unwrap()
}

#[test]
fn the_rendered_fragment_satisfies_the_windows_terminal_schema() {
    let fragment = fragment();
    assert!(validator().is_valid(&settings_with(&fragment["schemes"])));
}

#[test]
fn the_schema_check_rejects_a_renamed_magenta_role() {
    let mut fragment = fragment();
    let scheme = fragment["schemes"][0].as_object_mut().unwrap();
    let purple = scheme.remove("purple").unwrap();
    scheme.insert("magenta".to_owned(), purple);
    assert!(!validator().is_valid(&settings_with(&fragment["schemes"])));
}

#[test]
fn the_schema_check_rejects_an_alpha_colour() {
    let mut fragment = fragment();
    fragment["schemes"][0]["background"] = json!("#0b1018ff");
    assert!(!validator().is_valid(&settings_with(&fragment["schemes"])));
}

#[test]
fn the_settings_document_is_only_valid_with_all_three_root_keys() {
    let fragment = fragment();
    assert!(!validator().is_valid(&json!({ "schemes": fragment["schemes"] })));
}
