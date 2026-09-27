use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::Error;
use crate::jsonc::{self, Key};
use crate::model::ids::Target;
use crate::plan::{Absent, Plan, PlannedWrite};

const TEMP_SUFFIX: &str = ".onecoat.tmp";
const BACKUP_SUFFIX: &str = ".onecoat.bak";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WriteOutcome {
    Written,
    Unchanged,
}

#[derive(Debug)]
pub struct WriteReport {
    pub target: Target,
    pub path: PathBuf,
    pub outcome: WriteOutcome,
}

#[derive(Debug)]
pub struct ResolvedWrite {
    pub target: Target,
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub changed: Vec<Key>,
    pub outcome: WriteOutcome,
    pub backup: Option<PathBuf>,
}

pub fn resolve(plan: &Plan) -> Result<Vec<ResolvedWrite>, Error> {
    plan.writes.iter().map(resolve_one).collect()
}

pub fn preview(plan: &Plan) -> Result<Vec<String>, Error> {
    let mut lines = Vec::new();
    for write in resolve(plan)? {
        let verb = match write.outcome {
            WriteOutcome::Written => "would write",
            WriteOutcome::Unchanged => "unchanged",
        };
        lines.push(format!("{verb} {}", write.path.display()));
        for key in &write.changed {
            lines.push(format!("  {key}"));
        }
    }
    Ok(lines)
}

pub fn execute(plan: &Plan) -> Result<Vec<WriteReport>, Error> {
    let mut reports = Vec::new();
    for write in resolve(plan)? {
        if write.outcome == WriteOutcome::Unchanged {
            reports.push(WriteReport {
                target: write.target,
                path: write.path,
                outcome: WriteOutcome::Unchanged,
            });
            continue;
        }
        replace(&write)?;
        verify_written(&write.path, write.backup.as_deref())?;
        reports.push(WriteReport {
            target: write.target,
            path: write.path,
            outcome: WriteOutcome::Written,
        });
    }
    Ok(reports)
}

pub fn verify_written(path: &Path, backup: Option<&Path>) -> Result<(), Error> {
    let source = fs::read_to_string(path).map_err(|source| Error::FileUnreadable {
        path: path.to_path_buf(),
        source,
    })?;
    let Err(problem) = jsonc::verify(path, &source) else {
        return Ok(());
    };
    let Some(backup) = backup else {
        return Err(problem);
    };
    if !backup.exists() {
        return Err(Error::BackupMissing {
            path: path.to_path_buf(),
            backup: backup.to_path_buf(),
        });
    }
    restore(backup, path)?;
    Err(Error::ApplyNotVerified {
        path: path.to_path_buf(),
        backup: backup.to_path_buf(),
        source: Box::new(problem),
    })
}

fn resolve_one(planned: &PlannedWrite) -> Result<ResolvedWrite, Error> {
    let path = planned.path.as_path();
    let existing = read(path)?;
    let backup = existing
        .is_some()
        .then(|| append_to_file_name(path, BACKUP_SUFFIX));
    let (bytes, changed) = match &existing {
        Some(current) => {
            let source = std::str::from_utf8(current).map_err(|_| not_utf8(path))?;
            let spliced = jsonc::splice(path, source, &planned.edits)?;
            (spliced.text.into_bytes(), spliced.changed)
        }
        None => match planned.absent {
            Absent::Fail => {
                return Err(Error::MissingFile {
                    path: planned.path.clone(),
                });
            }
            Absent::Create => {
                let spliced = jsonc::splice(path, jsonc::NEW_DOCUMENT, &planned.edits)?;
                (spliced.text.into_bytes(), spliced.changed)
            }
        },
    };
    let outcome = match &existing {
        Some(current) if *current == bytes => WriteOutcome::Unchanged,
        _ => WriteOutcome::Written,
    };
    Ok(ResolvedWrite {
        target: planned.target,
        path: planned.path.clone(),
        bytes,
        changed,
        outcome,
        backup,
    })
}

fn replace(write: &ResolvedWrite) -> Result<(), Error> {
    let path = write.path.as_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::DirCreateFailed {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let temp = append_to_file_name(path, TEMP_SUFFIX);
    if let Err(source) = create_synced(&temp, &write.bytes) {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed { path: temp, source });
    }
    if let Some(backup) = &write.backup
        && let Err(source) = fs::copy(path, backup)
    {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed {
            path: backup.clone(),
            source,
        });
    }
    if let Err(source) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed {
            path: write.path.clone(),
            source,
        });
    }
    Ok(())
}

fn restore(backup: &Path, path: &Path) -> Result<(), Error> {
    let bytes = fs::read(backup).map_err(|source| Error::FileUnreadable {
        path: backup.to_path_buf(),
        source,
    })?;
    let temp = append_to_file_name(path, TEMP_SUFFIX);
    if let Err(source) = create_synced(&temp, &bytes) {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed { path: temp, source });
    }
    fs::rename(&temp, path).map_err(|source| Error::FileWriteFailed {
        path: path.to_path_buf(),
        source,
    })
}

fn read(path: &Path) -> Result<Option<Vec<u8>>, Error> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::FileUnreadable {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn not_utf8(path: &Path) -> Error {
    Error::FileUnreadable {
        path: path.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidData, "the file is not UTF-8"),
    }
}

fn create_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn append_to_file_name(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;
    use tempfile::TempDir;

    use super::{WriteOutcome, execute, preview, resolve, verify_written};
    use crate::Error;
    use crate::jsonc::{Edit, Key};
    use crate::model::ids::Target;
    use crate::plan::{Absent, Plan, PlannedWrite};

    fn one_write_plan(path: PathBuf, absent: Absent) -> Plan {
        Plan {
            writes: vec![PlannedWrite {
                target: Target::Wt,
                path,
                edits: vec![Edit::Set {
                    key: Key::parse("theme"),
                    value: json!("dark"),
                }],
                absent,
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
}
