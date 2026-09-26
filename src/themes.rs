//! Bundled themes and the theme set a run works with.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::model::ids::{Origin, ThemeId};
use crate::model::theme::{Theme, Validated};

pub(crate) const BUNDLED: &[(&str, &str)] = &[("nord", include_str!("../themes/nord.toml"))];

#[allow(missing_docs)]
#[derive(Debug)]
pub struct ThemeSet {
    themes: BTreeMap<ThemeId, Theme<Validated>>,
}

#[allow(missing_docs)]
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
mod tests {
    use std::fs;

    use super::{BUNDLED, ThemeSet};
    use crate::model::ids::Origin;
    use crate::model::theme::Theme;

    #[test]
    fn every_bundled_theme_parses_and_declares_its_registry_name() {
        assert!(!BUNDLED.is_empty());
        for (name, source) in BUNDLED {
            let theme = Theme::parse(
                std::path::PathBuf::from(format!("themes/{name}.toml")),
                source,
                Origin::Bundled,
            )
            .unwrap()
            .validate()
            .unwrap();
            assert_eq!(theme.id.as_str(), *name);
            assert_eq!(theme.origin, Origin::Bundled);
        }
    }

    #[test]
    fn a_missing_user_directory_loads_only_bundled_themes() {
        let dir = tempfile::tempdir().unwrap();
        let set = ThemeSet::load(&dir.path().join("absent")).unwrap();
        let ids: Vec<String> = set.iter().map(|theme| theme.id.to_string()).collect();
        let expected: Vec<String> = BUNDLED.iter().map(|(name, _)| (*name).to_owned()).collect();
        assert_eq!(ids, expected);
    }

    #[test]
    fn a_user_theme_shadows_the_bundled_theme() {
        let dir = tempfile::tempdir().unwrap();
        let themes = dir.path().join("themes");
        fs::create_dir(&themes).unwrap();
        fs::write(
            themes.join("shadow-nord.toml"),
            include_str!("../tests/fixtures/themes/shadow-nord.toml"),
        )
        .unwrap();

        let set = ThemeSet::load(&themes).unwrap();
        let nord = set.get("nord").unwrap();
        assert_eq!(nord.origin, Origin::User);
        assert_eq!(nord.name, "Shadow Nord");
        assert_eq!(
            nord.data.palette[crate::model::palette::Base16Entry::B00].to_string(),
            "#101820"
        );
        let count = set
            .iter()
            .filter(|theme| theme.id.as_str() == "nord")
            .count();
        assert_eq!(count, 1, "the bundled entry is replaced, not duplicated");
    }

    #[test]
    fn two_user_theme_files_with_one_id_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let themes = dir.path().join("themes");
        fs::create_dir(&themes).unwrap();
        let source = include_str!("../tests/fixtures/themes/shadow-nord.toml");
        fs::write(themes.join("a.toml"), source).unwrap();
        fs::write(themes.join("b.toml"), source).unwrap();

        let error = ThemeSet::load(&themes).unwrap_err();
        assert!(
            matches!(error, crate::Error::DuplicateThemeId { .. }),
            "{error}"
        );
        let message = error.to_string();
        assert!(message.contains("nord"), "{message}");
        assert!(
            message.contains("a.toml") && message.contains("b.toml"),
            "{message}"
        );
    }

    #[test]
    fn non_toml_entries_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let themes = dir.path().join("themes");
        fs::create_dir(&themes).unwrap();
        fs::create_dir(themes.join("nested")).unwrap();
        fs::write(themes.join("notes.txt"), "not a theme").unwrap();

        let set = ThemeSet::load(&themes).unwrap();
        let ids: Vec<&str> = set.iter().map(|theme| theme.id.as_str()).collect();
        assert_eq!(ids, ["nord"], "only the bundled theme remains");
    }
}
