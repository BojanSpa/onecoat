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
#[path = "../tests/unit/exec.rs"]
mod tests;
