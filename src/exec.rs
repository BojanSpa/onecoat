use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::Error;
use crate::model::ids::Target;
use crate::plan::{Plan, PlannedWrite};

const TEMP_SUFFIX: &str = ".onecoat.tmp";
const BACKUP_SUFFIX: &str = ".onecoat.bak";

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WriteOutcome {
    Written,
    Unchanged,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct WriteReport {
    pub target: Target,
    pub path: PathBuf,
    pub outcome: WriteOutcome,
}

#[allow(missing_docs)]
pub fn execute(plan: &Plan) -> Result<Vec<WriteReport>, Error> {
    plan.writes.iter().map(write_one).collect()
}

fn write_one(planned: &PlannedWrite) -> Result<WriteReport, Error> {
    let path = planned.path.as_path();
    let existing = match fs::read(path) {
        Ok(existing) => Some(existing),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(Error::FileUnreadable {
                path: planned.path.clone(),
                source,
            });
        }
    };
    if existing.as_deref() == Some(planned.bytes.as_slice()) {
        return Ok(WriteReport {
            target: planned.target,
            path: planned.path.clone(),
            outcome: WriteOutcome::Unchanged,
        });
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::DirCreateFailed {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let temp = append_to_file_name(path, TEMP_SUFFIX);
    let backup = append_to_file_name(path, BACKUP_SUFFIX);

    if let Err(source) = create_synced(&temp, &planned.bytes) {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed { path: temp, source });
    }
    if existing.is_some()
        && let Err(source) = fs::copy(path, &backup)
    {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed {
            path: backup,
            source,
        });
    }
    if let Err(source) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed {
            path: planned.path.clone(),
            source,
        });
    }
    Ok(WriteReport {
        target: planned.target,
        path: planned.path.clone(),
        outcome: WriteOutcome::Written,
    })
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

    use super::{WriteOutcome, execute};
    use crate::Error;
    use crate::model::ids::Target;
    use crate::plan::{Plan, PlannedWrite};

    fn one_write_plan(path: PathBuf, bytes: &[u8]) -> Plan {
        Plan {
            writes: vec![PlannedWrite {
                target: Target::Wt,
                path,
                bytes: bytes.to_vec(),
            }],
        }
    }

    #[test]
    fn the_previous_content_survives_a_failed_backup() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("schemes.json");
        fs::write(&target, b"previous").unwrap();
        let backup = dir.path().join("schemes.json.onecoat.bak");
        fs::create_dir(&backup).unwrap();

        let error = execute(&one_write_plan(target.clone(), b"next")).unwrap_err();
        assert!(
            matches!(&error, Error::FileWriteFailed { path, .. } if *path == backup),
            "{error}"
        );
        assert_eq!(fs::read(&target).unwrap(), b"previous");
        assert!(!dir.path().join("schemes.json.onecoat.tmp").exists());
    }

    #[test]
    fn matching_bytes_are_left_alone_without_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("schemes.json");
        fs::write(&target, b"same").unwrap();

        let reports = execute(&one_write_plan(target.clone(), b"same")).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].outcome, WriteOutcome::Unchanged);
        assert!(!dir.path().join("schemes.json.onecoat.bak").exists());
        assert!(!dir.path().join("schemes.json.onecoat.tmp").exists());
    }

    #[test]
    fn a_changed_file_is_backed_up_before_it_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir
            .path()
            .join("Fragments")
            .join("onecoat")
            .join("schemes.json");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"previous").unwrap();

        let reports = execute(&one_write_plan(target.clone(), b"next")).unwrap();
        assert_eq!(reports[0].outcome, WriteOutcome::Written);
        assert_eq!(fs::read(&target).unwrap(), b"next");
        assert_eq!(
            fs::read(target.with_extension("json.onecoat.bak")).unwrap(),
            b"previous"
        );
    }

    #[test]
    fn a_missing_file_is_created_without_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested").join("schemes.json");

        let reports = execute(&one_write_plan(target.clone(), b"first")).unwrap();
        assert_eq!(reports[0].outcome, WriteOutcome::Written);
        assert_eq!(fs::read(&target).unwrap(), b"first");
        assert!(
            !dir.path()
                .join("nested")
                .join("schemes.json.onecoat.bak")
                .exists()
        );
    }
}
