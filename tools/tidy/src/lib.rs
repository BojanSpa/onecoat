use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

pub mod checks;
pub mod scan;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read `{path}`: {source}")]
    Read { path: PathBuf, source: io::Error },

    #[error("cannot list `{path}`: {source}")]
    List { path: PathBuf, source: io::Error },

    #[error("`{path}` is not valid UTF-8")]
    NotUtf8 { path: PathBuf },

    #[error("`{path}` is not a directory")]
    RootMissing { path: PathBuf },

    #[error(
        "`{path}` is missing; a check depends on it, so move the check with the file and update docs/architecture.md"
    )]
    CheckTargetMissing { path: PathBuf },
}

pub trait Check {
    fn id(&self) -> &'static str;

    fn run(&self, repo: &Repo) -> Result<Vec<Diagnostic>, Error>;
}

#[derive(Debug)]
pub struct Diagnostic {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub check: &'static str,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}: {}",
            self.path, self.line, self.column, self.check, self.message
        )
    }
}

pub struct Source {
    pub path: String,
    pub text: String,
}

pub struct Repo {
    root: PathBuf,
}

impl Repo {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn exists(&self, relative: &str) -> bool {
        self.root.join(relative).exists()
    }

    pub fn read(&self, relative: &str) -> Result<String, Error> {
        self.read_path(&self.root.join(relative))
    }

    pub fn sources(&self, entries: &[&str], extension: &str) -> Result<Vec<Source>, Error> {
        let mut paths = Vec::new();
        for entry in entries {
            collect(&self.root.join(entry), extension, &mut paths)?;
        }
        paths.sort();
        paths
            .into_iter()
            .map(|path| {
                let text = self.read_path(&path)?;
                Ok(Source {
                    path: self.relative(&path),
                    text,
                })
            })
            .collect()
    }

    fn read_path(&self, path: &Path) -> Result<String, Error> {
        let bytes = std::fs::read(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        String::from_utf8(bytes).map_err(|_| Error::NotUtf8 {
            path: path.to_path_buf(),
        })
    }

    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

pub fn run(root: &Path) -> Result<Vec<Diagnostic>, Error> {
    if !root.is_dir() {
        return Err(Error::RootMissing {
            path: root.to_path_buf(),
        });
    }
    let repo = Repo::new(root);
    let results = checks::CHECKS
        .iter()
        .map(|check| check.run(&repo))
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(results.into_iter().flatten().collect())
}

fn collect(path: &Path, extension: &str, found: &mut Vec<PathBuf>) -> Result<(), Error> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        let entries = std::fs::read_dir(path).map_err(|source| Error::List {
            path: path.to_path_buf(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| Error::List {
                path: path.to_path_buf(),
                source,
            })?;
            collect(&entry.path(), extension, found)?;
        }
        Ok(())
    } else if path.extension().is_some_and(|found| found == extension) {
        found.push(path.to_path_buf());
        Ok(())
    } else {
        Ok(())
    }
}
