use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

use crate::Error;
use crate::jsonc::{self, Key};
use crate::render::wt;
use crate::targets::{HERDR_PROGRAM, Paths};

const COLUMNS: usize = 4;
const NO_VALUE: &str = "-";
const KIND_PATH: &str = "path";
const KIND_BINARY: &str = "binary";
const KIND_SOCKET: &str = "socket";
const KIND_PIN: &str = "pin";
const KIND_STATE: &str = "state";
const PRESENT: &str = "present";
const MISSING: &str = "missing";
const FOUND: &str = "found";
const AGE_NAME: &str = "age";
const JUST_NOW: &str = "just now";
const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const PATH_EXTENSIONS: [&str; 4] = [".COM", ".EXE", ".BAT", ".CMD"];

#[derive(Clone, Copy)]
enum EntryKind {
    File,
    Directory,
}

impl EntryKind {
    fn present(self, path: &Path) -> bool {
        let Ok(metadata) = path.metadata() else {
            return false;
        };

        match self {
            Self::File => metadata.is_file(),
            Self::Directory => metadata.is_dir(),
        }
    }
}

struct PathRow<'a> {
    name: &'static str,
    path: &'a Path,
    present: bool,
}

struct BinaryRow {
    name: &'static str,
    path: Option<PathBuf>,
}

struct SocketRow<'a> {
    name: &'static str,
    path: &'a Path,
    present: bool,
}

struct PinRow<'a> {
    profile: String,
    scheme: String,
    path: &'a Path,
}

pub struct Report<'a> {
    paths: Vec<PathRow<'a>>,
    binaries: Vec<BinaryRow>,
    sockets: Vec<SocketRow<'a>>,
    pins: Vec<PinRow<'a>>,
    age_seconds: Option<u64>,
}

#[derive(Serialize)]
struct Document<'a> {
    paths: Vec<PathJson<'a>>,
    binaries: Vec<BinaryJson<'a>>,
    sockets: Vec<SocketJson<'a>>,
    pins: Vec<PinJson<'a>>,
    state: StateJson,
}

#[derive(Serialize)]
struct PathJson<'a> {
    name: &'a str,
    path: String,
    present: bool,
}

#[derive(Serialize)]
struct BinaryJson<'a> {
    name: &'a str,
    found: bool,
    path: Option<String>,
}

#[derive(Serialize)]
struct SocketJson<'a> {
    name: &'a str,
    path: String,
    present: bool,
}

#[derive(Serialize)]
struct PinJson<'a> {
    profile: &'a str,
    scheme: &'a str,
}

#[derive(Serialize)]
struct StateJson {
    age_seconds: Option<u64>,
}

impl<'a> PathJson<'a> {
    fn of(row: &'a PathRow<'_>) -> Self {
        Self {
            name: row.name,
            path: row.path.display().to_string(),
            present: row.present,
        }
    }
}

impl<'a> BinaryJson<'a> {
    fn of(row: &'a BinaryRow) -> Self {
        Self {
            name: row.name,
            found: row.path.is_some(),
            path: row.path.as_deref().map(|path| path.display().to_string()),
        }
    }
}

impl<'a> SocketJson<'a> {
    fn of(row: &'a SocketRow<'_>) -> Self {
        Self {
            name: row.name,
            path: row.path.display().to_string(),
            present: row.present,
        }
    }
}

impl<'a> PinJson<'a> {
    fn of(row: &'a PinRow<'_>) -> Self {
        Self {
            profile: &row.profile,
            scheme: &row.scheme,
        }
    }
}

pub fn probe(paths: &Paths) -> Result<Report<'_>, Error> {
    let entries = [
        ("wt fragment", &paths.wt_fragment, EntryKind::File),
        ("wt settings", &paths.wt_settings, EntryKind::File),
        ("herdr config", &paths.herdr_config, EntryKind::File),
        ("omp themes", &paths.omp_themes, EntryKind::Directory),
        ("omp config", &paths.omp_config, EntryKind::File),
        ("user themes", &paths.user_themes, EntryKind::Directory),
        ("state", &paths.state, EntryKind::File),
    ];

    let rows = entries
        .iter()
        .map(|&(name, path, kind)| PathRow {
            name,
            path,
            present: kind.present(path),
        })
        .collect();

    Ok(Report {
        paths: rows,
        binaries: vec![BinaryRow {
            name: HERDR_PROGRAM,
            path: find_program(HERDR_PROGRAM),
        }],
        sockets: vec![SocketRow {
            name: HERDR_PROGRAM,
            path: &paths.herdr_socket,
            present: paths.herdr_socket.exists(),
        }],
        pins: pins(paths)?,
        age_seconds: state_age(&paths.state)?,
    })
}

impl Report<'_> {
    pub fn table(&self) -> String {
        let rows = self.rows();

        let widths: Vec<usize> = (0..COLUMNS)
            .map(|index| {
                rows.iter()
                    .map(|row| row[index].chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();

        rows.iter()
            .map(|row| pad_row(row, &widths))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn json(&self) -> Result<String, Error> {
        let document = Document {
            paths: self.paths.iter().map(PathJson::of).collect(),
            binaries: self.binaries.iter().map(BinaryJson::of).collect(),
            sockets: self.sockets.iter().map(SocketJson::of).collect(),
            pins: self.pins.iter().map(PinJson::of).collect(),
            state: StateJson {
                age_seconds: self.age_seconds,
            },
        };

        serde_json::to_string_pretty(&document).map_err(|source| Error::JsonEncodeFailed { source })
    }

    fn rows(&self) -> Vec<[String; COLUMNS]> {
        let mut rows = vec![[
            "kind".to_owned(),
            "name".to_owned(),
            "path".to_owned(),
            "status".to_owned(),
        ]];

        for row in &self.paths {
            rows.push([
                KIND_PATH.to_owned(),
                row.name.to_owned(),
                row.path.display().to_string(),
                presence(row.present).to_owned(),
            ]);
        }

        for row in &self.binaries {
            let (status, path) = match &row.path {
                Some(path) => (FOUND, path.display().to_string()),
                None => (MISSING, NO_VALUE.to_owned()),
            };

            rows.push([
                KIND_BINARY.to_owned(),
                row.name.to_owned(),
                path,
                status.to_owned(),
            ]);
        }

        for row in &self.sockets {
            rows.push([
                KIND_SOCKET.to_owned(),
                row.name.to_owned(),
                row.path.display().to_string(),
                presence(row.present).to_owned(),
            ]);
        }

        for row in &self.pins {
            rows.push([
                KIND_PIN.to_owned(),
                row.profile.clone(),
                row.path.display().to_string(),
                row.scheme.clone(),
            ]);
        }

        rows.push([
            KIND_STATE.to_owned(),
            AGE_NAME.to_owned(),
            NO_VALUE.to_owned(),
            self.age_seconds
                .map_or_else(|| NO_VALUE.to_owned(), age_text),
        ]);

        rows
    }
}

fn presence(present: bool) -> &'static str {
    if present { PRESENT } else { MISSING }
}

fn pad_row(row: &[String; COLUMNS], widths: &[usize]) -> String {
    row.iter()
        .zip(widths)
        .map(|(cell, width)| format!("{cell:<width$}"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end()
        .to_owned()
}

fn pins(paths: &Paths) -> Result<Vec<PinRow<'_>>, Error> {
    let path = &paths.wt_settings;

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(Error::FileUnreadable {
                path: path.clone(),
                source,
            });
        }
    };

    let key = Key::parse(wt::PROFILES_KEY);
    let found = jsonc::pinned(path, &source, &key, wt::COLOR_SCHEME_FIELD)?;

    Ok(found
        .into_iter()
        .map(|pin| PinRow {
            profile: pin.name,
            scheme: pin.scheme,
            path,
        })
        .collect())
}

fn state_age(path: &Path) -> Result<Option<u64>, Error> {
    let modified = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            metadata
                .modified()
                .map_err(|source| Error::FileUnreadable {
                    path: path.to_path_buf(),
                    source,
                })?
        }
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(Error::FileUnreadable {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    Ok(Some(age_seconds(modified, SystemTime::now())))
}

fn age_seconds(modified: SystemTime, now: SystemTime) -> u64 {
    now.duration_since(modified).unwrap_or_default().as_secs()
}

fn age_text(seconds: u64) -> String {
    if seconds < MINUTE {
        return JUST_NOW.to_owned();
    }

    if seconds < HOUR {
        return format!("{}m", seconds / MINUTE);
    }

    if seconds < DAY {
        let minutes = (seconds % HOUR) / MINUTE;

        return if minutes == 0 {
            format!("{}h", seconds / HOUR)
        } else {
            format!("{}h {minutes}m", seconds / HOUR)
        };
    }

    let hours = (seconds % DAY) / HOUR;
    if hours == 0 {
        format!("{}d", seconds / DAY)
    } else {
        format!("{}d {hours}h", seconds / DAY)
    }
}

fn find_program(program: &str) -> Option<PathBuf> {
    let search = env::var_os("PATH")?;
    let extensions = path_extensions();
    for directory in env::split_paths(&search) {
        let bare = directory.join(program);
        if bare.is_file() {
            return Some(bare);
        }

        for extension in &extensions {
            let candidate = directory.join(format!("{program}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn path_extensions() -> Vec<String> {
    let set: Vec<String> = env::var("PATHEXT")
        .unwrap_or_default()
        .split(';')
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect();

    if set.is_empty() {
        default_extensions()
    } else {
        set
    }
}

fn default_extensions() -> Vec<String> {
    PATH_EXTENSIONS
        .iter()
        .map(|extension| (*extension).to_owned())
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/doctor.rs"]
mod tests;
