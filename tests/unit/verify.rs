use std::path::{Path, PathBuf};

use serde_json::json;

use super::{Alternatives, Finding, drift, shared_alternatives};
use crate::Error;
use crate::jsonc::{Edit, Fill, Key};
use crate::model::ids::{Origin, Slot, Target};
use crate::model::theme::{Theme, Validated};
use crate::plan::{Absent, Content, PlannedWrite};
use crate::render::{herdr, omp};

fn write(target: Target, content: Content, absent: Absent) -> PlannedWrite {
    PlannedWrite {
        target,
        path: PathBuf::from("target/file"),
        content,
        absent,
        pins: None,
    }
}

fn no_alternatives() -> Alternatives {
    shared_alternatives(std::iter::empty())
}

fn theme(source: &str) -> Theme<Validated> {
    Theme::parse(PathBuf::from("themes/nord.toml"), source, Origin::Bundled)
        .unwrap()
        .validate()
        .unwrap()
}

fn nord() -> Theme<Validated> {
    theme(crate::themes::BUNDLED[0].1)
}

fn herdr_overrides() -> Theme<Validated> {
    theme(&format!(
        "{}\n[targets.herdr]\nname = \"nord\"\n",
        crate::themes::BUNDLED[0].1
    ))
}

fn herdr_write(slot: Slot, theme: &Theme<Validated>) -> PlannedWrite {
    write(
        Target::Herdr,
        Content::Toml(herdr::edits(theme, slot).unwrap()),
        Absent::Fail,
    )
}

fn applied_herdr_document(themes: &[(Slot, &Theme<Validated>)]) -> String {
    let mut text = "onboarding = false\n".to_owned();
    for (slot, theme) in themes {
        let edits = herdr::edits(theme, *slot).unwrap();

        text = herdr::splice(Path::new("config.toml"), &text, &edits)
            .unwrap()
            .text;
    }

    text
}

fn two_slot_herdr() -> (PlannedWrite, Alternatives, String) {
    let (dark, light) = (nord(), herdr_overrides());

    let writes = [
        herdr_write(Slot::Dark, &dark),
        herdr_write(Slot::Light, &light),
    ];

    let alternatives = shared_alternatives(writes.iter());
    let applied = applied_herdr_document(&[(Slot::Dark, &dark), (Slot::Light, &light)]);

    (writes.into_iter().next().unwrap(), alternatives, applied)
}

#[test]
fn a_reformatted_document_is_not_drift() {
    let edit = Edit::Set {
        key: Key::parse("theme"),
        value: json!("dark"),
    };

    let source = "{\n    // windows terminal writes comments\n    \"theme\" : \"dark\",\n}\n";

    let findings = drift(
        Slot::Dark,
        &write(Target::Wt, Content::Jsonc(vec![edit]), Absent::Fail),
        Some(source),
        &no_alternatives(),
    )
    .unwrap();

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn an_edited_colour_reports_its_full_key() {
    let edit = Edit::Element {
        key: Key::parse("schemes"),
        name: "onecoat-dark".to_owned(),
        value: json!({"name": "onecoat-dark", "background": "#0b1018"}),
    };

    let source = r##"{"schemes": [{"name": "onecoat-dark", "background": "#0b1019"}]}"##;

    let findings = drift(
        Slot::Dark,
        &write(Target::Wt, Content::Jsonc(vec![edit]), Absent::Fail),
        Some(source),
        &no_alternatives(),
    )
    .unwrap();

    assert_eq!(
        findings,
        [Finding {
            target: Target::Wt,
            slot: Slot::Dark,
            key: "schemes.onecoat-dark.background".to_owned(),
            expected: Some("#0b1018".to_owned()),
            found: Some("#0b1019".to_owned()),
        }]
    );
}

#[test]
fn a_missing_pair_side_reports_the_side() {
    let edit = Edit::Pair {
        key: Key::parse("theme"),
        side: Slot::Dark,
        name: "onecoat-dark".to_owned(),
        fill: Fill::BuiltIn,
    };

    let source = r#"{"theme": {"light": "One Half Light"}}"#;

    let findings = drift(
        Slot::Dark,
        &write(Target::Wt, Content::Jsonc(vec![edit]), Absent::Fail),
        Some(source),
        &no_alternatives(),
    )
    .unwrap();

    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].key, "theme.dark");
    assert_eq!(findings[0].expected, Some("onecoat-dark".to_owned()));
    assert_eq!(findings[0].found, None);
}

#[test]
fn an_absent_creatable_file_reports_every_owned_key() {
    let generated =
        "{\n  \"colors\": {\n    \"text\": \"#dadbdd\"\n  },\n  \"name\": \"onecoat-dark\"\n}\n";

    let findings = drift(
        Slot::Dark,
        &write(
            Target::Omp,
            Content::Generated(generated.to_owned()),
            Absent::Create,
        ),
        None,
        &no_alternatives(),
    )
    .unwrap();

    let keys: Vec<&str> = findings
        .iter()
        .map(|finding| finding.key.as_str())
        .collect();

    assert_eq!(keys, ["colors.text", "name"]);
    assert!(findings.iter().all(|finding| finding.found.is_none()));
}

#[test]
fn an_absent_creatable_jsonc_file_reports_every_owned_key() {
    let edit = Edit::Element {
        key: Key::parse("schemes"),
        name: "onecoat-dark".to_owned(),
        value: json!({"name": "onecoat-dark", "background": "#0b1018"}),
    };

    let findings = drift(
        Slot::Dark,
        &write(Target::Wt, Content::Jsonc(vec![edit]), Absent::Create),
        None,
        &no_alternatives(),
    )
    .unwrap();

    let keys: Vec<&str> = findings
        .iter()
        .map(|finding| finding.key.as_str())
        .collect();

    assert_eq!(
        keys,
        [
            "schemes.onecoat-dark.background",
            "schemes.onecoat-dark.name"
        ]
    );

    assert!(findings.iter().all(|finding| finding.found.is_none()));
}

#[test]
fn an_absent_required_file_is_an_error() {
    let planned = write(
        Target::Omp,
        Content::Yaml(omp::config_edits(Slot::Dark)),
        Absent::Fail,
    );

    let error = drift(Slot::Dark, &planned, None, &no_alternatives()).unwrap_err();

    assert!(
        matches!(&error, Error::MissingFile { path } if *path == planned.path),
        "{error}"
    );
}

#[test]
fn a_repoint_edit_yields_nothing() {
    let edit = Edit::Repoint {
        key: Key::parse("profiles.list"),
        field: "colorScheme".to_owned(),
        side: Slot::Dark,
        name: "onecoat-dark".to_owned(),
        fill: Fill::FromString,
    };

    let source =
        r#"{"profiles": {"list": [{"name": "PowerShell", "colorScheme": "One Half Dark"}]}}"#;

    let findings = drift(
        Slot::Dark,
        &write(Target::Wt, Content::Jsonc(vec![edit]), Absent::Fail),
        Some(source),
        &no_alternatives(),
    )
    .unwrap();

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn a_shared_herdr_token_matching_the_other_slot_is_not_drift() {
    let (dark_write, alternatives, applied) = two_slot_herdr();

    let findings = drift(Slot::Dark, &dark_write, Some(&applied), &alternatives).unwrap();

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn a_shared_herdr_token_matching_neither_slot_lists_both_values() {
    let (dark_write, alternatives, applied) = two_slot_herdr();

    let edited: String = applied
        .lines()
        .map(|line| {
            if line == "name = \"nord\"" {
                "name = \"bogus\""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let findings = drift(Slot::Dark, &dark_write, Some(&edited), &alternatives).unwrap();

    assert_eq!(
        findings,
        [Finding {
            target: Target::Herdr,
            slot: Slot::Dark,
            key: "theme.name".to_owned(),
            expected: Some("terminal or nord".to_owned()),
            found: Some("bogus".to_owned()),
        }]
    );
}

#[test]
fn an_extra_token_in_a_generated_file_reports_with_no_expected_value() {
    let generated =
        "{\n  \"colors\": {\n    \"text\": \"#dadbdd\"\n  },\n  \"name\": \"onecoat-dark\"\n}\n";

    let source = r##"{"name": "onecoat-dark", "colors": {"text": "#dadbdd", "bogus": "#ffffff"}}"##;

    let findings = drift(
        Slot::Dark,
        &write(
            Target::Omp,
            Content::Generated(generated.to_owned()),
            Absent::Create,
        ),
        Some(source),
        &no_alternatives(),
    )
    .unwrap();

    assert_eq!(
        findings,
        [Finding {
            target: Target::Omp,
            slot: Slot::Dark,
            key: "colors.bogus".to_owned(),
            expected: None,
            found: Some("#ffffff".to_owned()),
        }]
    );
}
