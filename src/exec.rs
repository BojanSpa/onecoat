//! The executor: the only code in the crate that touches files.
//!
//! Per write the order is: compare, create the parent directory, write a temporary file,
//! back up the previous content, then replace. Because the backup and the replace come
//! last, any failure leaves the target file byte-identical to what it was.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::Error;
use crate::model::ids::Target;
use crate::plan::{Plan, PlannedWrite};

/// What happened to one planned file.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WriteOutcome {
    Written,
    /// The file already held the planned bytes; neither the file nor its backup was
    /// touched (R-9).
    Unchanged,
}

/// The result of executing one planned write.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct WriteReport {
    pub target: Target,
    pub path: PathBuf,
    pub outcome: WriteOutcome,
}

/// Executes every write of the plan in order; the first failure stops it.
pub fn execute(plan: &Plan) -> Result<Vec<WriteReport>, Error> {
    plan.writes.iter().map(write_one).collect()
}

/// Writes one file, or reports it as unchanged (R-9).
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

    let temp = sibling(path, ".onecoat.tmp");
    let backup = sibling(path, ".onecoat.bak");

    if let Err(source) = write_new(&temp, &planned.bytes) {
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

/// Creates `path` and writes `bytes`, synced to disk before it is renamed.
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// `<path>` with `suffix` appended to the whole file name.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
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

    /// A one-write plan for `path`.
    fn plan_for(path: PathBuf, bytes: &[u8]) -> Plan {
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

        let error = execute(&plan_for(target.clone(), b"next")).unwrap_err();
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

        let reports = execute(&plan_for(target.clone(), b"same")).unwrap();
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

        let reports = execute(&plan_for(target.clone(), b"next")).unwrap();
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

        let reports = execute(&plan_for(target.clone(), b"first")).unwrap();
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
