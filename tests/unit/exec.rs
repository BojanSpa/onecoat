use std::fs;
use std::path::PathBuf;

use serde_json::json;
use tempfile::TempDir;

use super::{WriteOutcome, execute, preview, resolve, verify_written};
use crate::Error;
use crate::jsonc::{Edit, Key};
use crate::model::ids::{Slot, Target};
use crate::plan::{Absent, Content, Plan, PlannedWrite};
use crate::render::omp;

fn one_write_plan(path: PathBuf, absent: Absent) -> Plan {
    Plan {
        writes: vec![PlannedWrite {
            target: Target::Wt,
            path,
            content: Content::Jsonc(vec![Edit::Set {
                key: Key::parse("theme"),
                value: json!("dark"),
            }]),
            absent,
            pins: None,
        }],
    }
}

fn generated_plan(path: PathBuf) -> Plan {
    Plan {
        writes: vec![PlannedWrite {
            target: Target::Omp,
            path,
            content: Content::Generated("{\n  \"name\": \"onecoat-dark\"\n}\n".to_owned()),
            absent: Absent::Create,
            pins: None,
        }],
    }
}

fn yaml_plan(path: PathBuf) -> Plan {
    Plan {
        writes: vec![PlannedWrite {
            target: Target::Omp,
            path,
            content: Content::Yaml(omp::config_edits(Slot::Dark)),
            absent: Absent::Fail,
            pins: None,
        }],
    }
}

fn sandbox() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("settings.json");
    (dir, target)
}

#[test]
fn the_previous_content_survives_a_failed_backup() {
    let (dir, target) = sandbox();
    fs::write(&target, "{\n    \"a\": 1\n}\n").unwrap();
    let backup = dir.path().join("settings.json.onecoat.bak");
    fs::create_dir(&backup).unwrap();

    let error = execute(&one_write_plan(target.clone(), Absent::Fail)).unwrap_err();

    assert!(
        matches!(&error, Error::FileWriteFailed { path, .. } if *path == backup),
        "{error}"
    );

    assert_eq!(fs::read_to_string(&target).unwrap(), "{\n    \"a\": 1\n}\n");
    assert!(!dir.path().join("settings.json.onecoat.tmp").exists());
}

#[test]
fn a_document_that_already_holds_the_value_is_left_alone() {
    let (dir, target) = sandbox();
    fs::write(&target, "{\n  \"theme\": \"dark\"\n}\n").unwrap();

    let reports = execute(&one_write_plan(target.clone(), Absent::Fail)).unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, WriteOutcome::Unchanged);
    assert!(!dir.path().join("settings.json.onecoat.bak").exists());
    assert!(!dir.path().join("settings.json.onecoat.tmp").exists());

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "{\n  \"theme\": \"dark\"\n}\n"
    );
}

#[test]
fn a_changed_file_is_backed_up_before_it_is_replaced() {
    let (dir, target) = sandbox();
    fs::write(&target, "{\n  \"theme\": \"light\"\n}\n").unwrap();

    let reports = execute(&one_write_plan(target.clone(), Absent::Fail)).unwrap();
    assert_eq!(reports[0].outcome, WriteOutcome::Written);

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "{\n  \"theme\": \"dark\"\n}\n"
    );

    assert_eq!(
        fs::read_to_string(dir.path().join("settings.json.onecoat.bak")).unwrap(),
        "{\n  \"theme\": \"light\"\n}\n"
    );
}

#[test]
fn a_missing_file_is_created_without_a_backup() {
    let (dir, target) = sandbox();

    let reports = execute(&one_write_plan(target.clone(), Absent::Create)).unwrap();
    assert_eq!(reports[0].outcome, WriteOutcome::Written);

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "{\n  \"theme\": \"dark\"\n}\n"
    );

    assert!(!dir.path().join("settings.json.onecoat.bak").exists());
}

#[test]
fn a_missing_file_that_must_not_be_created_fails_the_apply() {
    let (dir, target) = sandbox();

    let error = execute(&one_write_plan(target.clone(), Absent::Fail)).unwrap_err();

    assert!(
        matches!(&error, Error::MissingFile { path } if *path == target),
        "{error}"
    );

    assert!(!target.exists());
    assert!(!dir.path().join("settings.json.onecoat.tmp").exists());
}

#[test]
fn a_corrupted_result_is_restored_from_its_backup() {
    let (dir, target) = sandbox();
    fs::write(&target, "{\"theme\": ").unwrap();
    let backup = dir.path().join("settings.json.onecoat.bak");
    fs::write(&backup, "{\"theme\": \"light\"}").unwrap();

    let error = verify_written(&target, Some(&backup)).unwrap_err();

    assert!(
        matches!(&error, Error::ApplyNotVerified { path, backup: kept, .. } if *path == target && *kept == backup),
        "{error}"
    );

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "{\"theme\": \"light\"}"
    );

    assert!(!dir.path().join("settings.json.onecoat.tmp").exists());
}

#[test]
fn a_corrupted_result_without_a_backup_names_the_file() {
    let (_dir, target) = sandbox();
    fs::write(&target, "{\"theme\": ").unwrap();

    let error = verify_written(&target, None).unwrap_err();

    assert!(
        matches!(&error, Error::JsoncUnparseable { path, .. } if *path == target),
        "{error}"
    );

    assert_eq!(fs::read_to_string(&target).unwrap(), "{\"theme\": ");
}

#[test]
fn a_corrupted_result_without_the_backup_file_says_so() {
    let (dir, target) = sandbox();
    fs::write(&target, "{\"theme\": ").unwrap();
    let backup = dir.path().join("settings.json.onecoat.bak");

    let error = verify_written(&target, Some(&backup)).unwrap_err();

    assert!(
        matches!(&error, Error::BackupMissing { path, backup: kept } if *path == target && *kept == backup),
        "{error}"
    );

    assert_eq!(fs::read_to_string(&target).unwrap(), "{\"theme\": ");
}

#[test]
fn a_parsable_file_passes_verification() {
    let (_dir, target) = sandbox();
    fs::write(&target, "{\n  // a comment\n  \"theme\": \"dark\"\n}\n").unwrap();

    assert!(verify_written(&target, None).is_ok());
}

#[test]
fn preview_names_every_file_and_every_changed_key_without_writing() {
    let (dir, target) = sandbox();
    fs::write(&target, "{\n    \"a\": 1\n}\n").unwrap();
    let before = fs::read(&target).unwrap();

    let plan = one_write_plan(target.clone(), Absent::Fail);
    let lines = preview(&plan).unwrap();

    assert_eq!(
        lines,
        [
            format!("would write {}", target.display()),
            "  theme".to_owned(),
        ]
    );

    assert_eq!(fs::read(&target).unwrap(), before);
    assert!(!dir.path().join("settings.json.onecoat.bak").exists());

    let resolved = resolve(&plan).unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].outcome, WriteOutcome::Written);
    assert_eq!(resolved[0].changed, [Key::parse("theme")]);

    assert_eq!(
        String::from_utf8(resolved[0].bytes.clone()).unwrap(),
        "{\n    \"a\": 1,\n    \"theme\": \"dark\"\n}\n"
    );
}

#[test]
fn preview_reports_an_unchanged_file_as_unchanged() {
    let (_dir, target) = sandbox();
    fs::write(&target, "{\n  \"theme\": \"dark\"\n}\n").unwrap();

    let lines = preview(&one_write_plan(target.clone(), Absent::Fail)).unwrap();
    assert_eq!(lines, [format!("unchanged {}", target.display())]);
}

#[test]
fn the_file_keeps_its_line_endings_and_trailing_state() {
    let (_dir, target) = sandbox();
    fs::write(&target, "{\r\n    \"a\": 1\r\n}").unwrap();

    execute(&one_write_plan(target.clone(), Absent::Fail)).unwrap();

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "{\r\n    \"a\": 1,\r\n    \"theme\": \"dark\"\r\n}"
    );
}

#[test]
fn a_file_that_is_not_utf8_is_reported() {
    let (_dir, target) = sandbox();
    fs::write(&target, [0xff, 0xfe, 0x00]).unwrap();

    let error = execute(&one_write_plan(target.clone(), Absent::Fail)).unwrap_err();

    assert!(
        matches!(&error, Error::FileUnreadable { path, .. } if *path == target),
        "{error}"
    );
}

#[test]
fn resolving_a_missing_file_touches_nothing() {
    let (_dir, target) = sandbox();
    let resolved = resolve(&one_write_plan(target.clone(), Absent::Create)).unwrap();

    assert_eq!(resolved[0].target, Target::Wt);
    assert_eq!(resolved[0].path, target);
    assert_eq!(resolved[0].backup, None);
    assert!(!target.exists(), "resolving a plan writes nothing");
}

#[test]
fn a_generated_file_reports_no_keys_and_is_written_whole() {
    let (dir, target) = sandbox();

    let lines = preview(&generated_plan(target.clone())).unwrap();
    assert_eq!(lines, [format!("would write {}", target.display())]);

    let reports = execute(&generated_plan(target.clone())).unwrap();
    assert_eq!(reports[0].target, Target::Omp);
    assert_eq!(reports[0].outcome, WriteOutcome::Written);

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "{\n  \"name\": \"onecoat-dark\"\n}\n"
    );

    assert!(!dir.path().join("settings.json.onecoat.bak").exists());

    let again = execute(&generated_plan(target.clone())).unwrap();
    assert_eq!(again[0].outcome, WriteOutcome::Unchanged);
}

#[test]
fn a_yaml_pin_is_spliced_reported_and_backed_up() {
    let (dir, target) = sandbox();
    fs::write(&target, "model: x\ntheme:\n  dark: old\n").unwrap();

    let resolved = resolve(&yaml_plan(target.clone())).unwrap();
    assert_eq!(resolved[0].changed, [Key::parse("theme.dark")]);

    assert_eq!(
        String::from_utf8(resolved[0].bytes.clone()).unwrap(),
        "model: x\ntheme:\n  dark: onecoat-dark\n"
    );

    let reports = execute(&yaml_plan(target.clone())).unwrap();
    assert_eq!(reports[0].outcome, WriteOutcome::Written);

    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "model: x\ntheme:\n  dark: onecoat-dark\n"
    );

    assert_eq!(
        fs::read_to_string(dir.path().join("settings.json.onecoat.bak")).unwrap(),
        "model: x\ntheme:\n  dark: old\n"
    );

    assert_eq!(
        execute(&yaml_plan(target.clone())).unwrap()[0].outcome,
        WriteOutcome::Unchanged
    );
}
