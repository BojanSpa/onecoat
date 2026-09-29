use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Error;
use crate::jsonc::{self, Key};
use crate::model::ids::Target;
use crate::plan::{Absent, Content, PinReport, Plan, PlannedWrite};
use crate::render::{herdr, omp};
use crate::state::{Problem, State};
use crate::targets::{HERDR_PROGRAM, Paths};

const TEMP_SUFFIX: &str = ".onecoat.tmp";
const BACKUP_SUFFIX: &str = ".onecoat.bak";
const HERDR_NO_BINARY: &str = "herdr is not on PATH, so the config check was skipped";
const HERDR_INTERACTIVE: &str = "no herdr server socket, so herdr picks this up at its next launch";
const HERDR_RELOADED: &str = "reloaded herdr's config";

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
    pub pin_notes: Vec<String>,
}

#[derive(Debug)]
pub struct ResolvedWrite {
    pub target: Target,
    pub path: PathBuf,
    pub content: Content,
    pub bytes: Vec<u8>,
    pub changed: Vec<Key>,
    pub outcome: WriteOutcome,
    pub backup: Option<PathBuf>,
    pub pin_notes: Vec<String>,
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

        lines.extend(write.pin_notes);
    }

    Ok(lines)
}

pub fn execute(paths: &Paths, plan: &Plan) -> Result<Vec<WriteReport>, Error> {
    let mut reports = Vec::new();
    for mut write in resolve(plan)? {
        if write.outcome == WriteOutcome::Unchanged {
            reports.push(WriteReport {
                target: write.target,
                path: write.path,
                outcome: WriteOutcome::Unchanged,
                pin_notes: write.pin_notes,
            });

            continue;
        }

        let mut pin_notes = std::mem::take(&mut write.pin_notes);
        if write.target == Target::Herdr
            && let Some(note) = check_herdr(&write.path, &write.bytes)?
        {
            pin_notes.push(note);
        }

        replace(&write)?;
        verify_content(&write)?;

        if write.target == Target::Herdr
            && let Some(note) = reload_herdr(&paths.herdr_socket)
        {
            pin_notes.push(note);
        }

        reports.push(WriteReport {
            target: write.target,
            path: write.path,
            outcome: WriteOutcome::Written,
            pin_notes,
        });
    }

    Ok(reports)
}

fn check_herdr(path: &Path, bytes: &[u8]) -> Result<Option<String>, Error> {
    let Some(live) = herdr_issues(path)? else {
        return Ok(Some(HERDR_NO_BINARY.to_owned()));
    };

    let candidate = append_to_file_name(path, TEMP_SUFFIX);
    if let Err(source) = create_synced(&candidate, bytes) {
        let _ = fs::remove_file(&candidate);

        return Err(Error::FileWriteFailed {
            path: candidate,
            source,
        });
    }

    let checked = herdr_issues(&candidate)?;
    let _ = fs::remove_file(&candidate);

    let added: Vec<String> = checked
        .unwrap_or_default()
        .into_iter()
        .filter(|issue| !live.contains(issue))
        .collect();

    match added.is_empty() {
        true => Ok(None),
        false => Err(Error::HerdrCheckFailed {
            path: path.to_path_buf(),
            issues: added.join("; "),
        }),
    }
}

fn herdr_issues(config: &Path) -> Result<Option<Vec<String>>, Error> {
    let output = match Command::new(HERDR_PROGRAM)
        .args(["config", "check"])
        .env("HERDR_CONFIG_PATH", config)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::CommandFailed {
                program: HERDR_PROGRAM,
                source,
            });
        }
    };

    let mut issues: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .chain(String::from_utf8_lossy(&output.stderr).lines())
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("config:"))
        .map(str::to_owned)
        .collect();

    issues.sort();
    issues.dedup();

    Ok(Some(issues))
}

fn reload_herdr(socket: &Path) -> Option<String> {
    if !socket.exists() {
        return Some(HERDR_INTERACTIVE.to_owned());
    }

    match Command::new(HERDR_PROGRAM)
        .args(["server", "reload-config"])
        .output()
    {
        Ok(output) if output.status.success() => Some(HERDR_RELOADED.to_owned()),
        Ok(output) => Some(format!(
            "herdr did not reload: {}",
            first_line(&output.stderr)
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Some(HERDR_NO_BINARY.to_owned()),
        Err(source) => Some(format!("cannot run `{HERDR_PROGRAM}`: {source}")),
    }
}

fn first_line(text: &[u8]) -> String {
    let text = String::from_utf8_lossy(text);

    text.lines()
        .find(|line| !line.trim().is_empty())
        .map_or_else(|| "no message".to_owned(), |line| line.trim().to_owned())
}

pub fn verify_written(path: &Path, backup: Option<&Path>) -> Result<(), Error> {
    let source = read_text(path)?;
    match jsonc::verify(path, &source) {
        Ok(()) => Ok(()),
        Err(problem) => restore_and_fail(path, backup, problem),
    }
}

fn verify_content(write: &ResolvedWrite) -> Result<(), Error> {
    let path = write.path.as_path();
    let source = read_text(path)?;

    let problem = match &write.content {
        Content::Jsonc(_) | Content::Generated(_) => jsonc::verify(path, &source).err(),
        Content::Toml(_) => herdr::splice(path, &source, &[]).err(),
        Content::Yaml(edits) => omp::splice_config(path, &source, edits).err(),
    };

    match problem {
        None => Ok(()),
        Some(problem) => restore_and_fail(path, write.backup.as_deref(), problem),
    }
}

fn restore_and_fail(path: &Path, backup: Option<&Path>, problem: Error) -> Result<(), Error> {
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

fn read_text(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).map_err(|source| Error::FileUnreadable {
        path: path.to_path_buf(),
        source,
    })
}

pub fn read_state(path: &Path) -> Result<State, Error> {
    let Some(bytes) = read(path)? else {
        return Ok(State::none());
    };

    let source = std::str::from_utf8(&bytes).map_err(|_| not_utf8(path))?;

    State::parse(source).map_err(|problem| state_error(path, problem))
}

pub fn write_state(path: &Path, state: &State) -> Result<(), Error> {
    let text = state.render()?;
    if read(path)?.as_deref() == Some(text.as_bytes()) {
        return Ok(());
    }

    write_bytes(path, text.as_bytes(), None)?;

    read_state(path).map(|_| ())
}

fn state_error(path: &Path, problem: Problem) -> Error {
    Error::StateInvalid {
        path: path.to_path_buf(),
        problem,
    }
}

fn resolve_one(planned: &PlannedWrite) -> Result<ResolvedWrite, Error> {
    let path = planned.path.as_path();
    let existing = read(path)?;

    let backup = existing
        .is_some()
        .then(|| append_to_file_name(path, BACKUP_SUFFIX));

    let existing_text = existing
        .as_deref()
        .map(|current| std::str::from_utf8(current).map_err(|_| not_utf8(path)))
        .transpose()?;

    let text = match (&existing_text, planned.absent) {
        (Some(source), _) => *source,
        (None, Absent::Fail) => {
            return Err(Error::MissingFile {
                path: planned.path.clone(),
            });
        }
        (None, Absent::Create) => jsonc::NEW_DOCUMENT,
    };

    let (bytes, changed) = match &planned.content {
        Content::Jsonc(edits) => {
            let spliced = jsonc::splice(path, text, edits)?;
            (spliced.text.into_bytes(), spliced.changed)
        }
        Content::Toml(edits) => {
            let spliced = herdr::splice(path, text, edits)?;
            (spliced.text.into_bytes(), spliced.changed)
        }
        Content::Yaml(edits) => {
            let spliced = omp::splice_config(path, text, edits)?;
            (spliced.text.into_bytes(), spliced.changed)
        }
        Content::Generated(generated) => (generated.clone().into_bytes(), Vec::new()),
    };

    let pin_notes = match (existing_text, &planned.pins) {
        (Some(source), Some(report)) => report_pins(path, source, report)?,
        _ => Vec::new(),
    };

    let outcome = match &existing {
        Some(current) if *current == bytes => WriteOutcome::Unchanged,
        _ => WriteOutcome::Written,
    };

    Ok(ResolvedWrite {
        target: planned.target,
        path: planned.path.clone(),
        content: planned.content.clone(),
        bytes,
        changed,
        outcome,
        backup,
        pin_notes,
    })
}

fn report_pins(path: &Path, source: &str, report: &PinReport) -> Result<Vec<String>, Error> {
    let (key, field) = match report {
        PinReport::Report { key, field } | PinReport::Repoint { key, field } => (key, field),
    };

    let notes = jsonc::pinned(path, source, key, field)?;

    Ok(notes
        .into_iter()
        .map(|pin| match report {
            PinReport::Report { .. } => format!("profile \"{}\" pins \"{}\"", pin.name, pin.scheme),
            PinReport::Repoint { .. } => {
                format!("repointed profile \"{}\" from \"{}\"", pin.name, pin.scheme)
            }
        })
        .collect())
}

fn replace(write: &ResolvedWrite) -> Result<(), Error> {
    write_bytes(&write.path, &write.bytes, write.backup.as_deref())
}

fn write_bytes(path: &Path, bytes: &[u8], backup: Option<&Path>) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::DirCreateFailed {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let temp = append_to_file_name(path, TEMP_SUFFIX);
    if let Err(source) = create_synced(&temp, bytes) {
        let _ = fs::remove_file(&temp);
        return Err(Error::FileWriteFailed { path: temp, source });
    }

    if let Some(backup) = backup
        && let Err(source) = fs::copy(path, backup)
    {
        let _ = fs::remove_file(&temp);

        return Err(Error::FileWriteFailed {
            path: backup.to_path_buf(),
            source,
        });
    }

    if let Err(source) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);

        return Err(Error::FileWriteFailed {
            path: path.to_path_buf(),
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
