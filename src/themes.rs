use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::model::ids::{Origin, ThemeId};
use crate::model::theme::{Theme, Validated};

pub(crate) const BUNDLED: &[(&str, &str)] = &[("nord", include_str!("../themes/nord.toml"))];

#[derive(Debug)]
pub struct ThemeSet {
    themes: BTreeMap<ThemeId, Theme<Validated>>,
}

impl ThemeSet {
    pub fn load(user_themes_dir: &Path) -> Result<Self, Error> {
        let mut themes = BTreeMap::new();
        for (name, source) in BUNDLED {
            let theme = Theme::parse(
                PathBuf::from(format!("themes/{name}.toml")),
                source,
                Origin::Bundled,
            )?
            .validate()?;
            themes.insert(theme.id.clone(), theme);
        }

        let mut declared: BTreeMap<ThemeId, PathBuf> = BTreeMap::new();
        for path in toml_files_by_name(user_themes_dir)? {
            let source = fs::read_to_string(&path).map_err(|source| Error::FileUnreadable {
                path: path.clone(),
                source,
            })?;
            let theme = Theme::parse(path.clone(), &source, Origin::User)?.validate()?;
            if let Some(other) = declared.insert(theme.id.clone(), path.clone()) {
                return Err(Error::DuplicateThemeId {
                    path,
                    other,
                    id: theme.id.clone(),
                });
            }
            themes.insert(theme.id.clone(), theme);
        }
        Ok(Self { themes })
    }

    pub fn get(&self, id: &str) -> Option<&Theme<Validated>> {
        self.themes.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Theme<Validated>> {
        self.themes.values()
    }
}

fn toml_files_by_name(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(Error::FileUnreadable {
                path: dir.to_path_buf(),
                source,
            });
        }
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::FileUnreadable {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let is_toml = path
            .extension()
            .is_some_and(|extension| extension == "toml");
        if is_toml && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            files.push(path);
        }
    }
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}

#[cfg(test)]
#[path = "../tests/unit/themes.rs"]
mod tests;
