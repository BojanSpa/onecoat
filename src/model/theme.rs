//! The theme document and its two phases: [`Parsed`] and [`Validated`].
//!
//! [`Theme::parse`] checks the *shape* of the document: which keys exist, and whether
//! each one has the TOML type the schema requires. Everything that needs a value to be
//! interpreted — colours, completeness, luminance, reserved ids — is
//! [`validate`](crate::validate)'s job, so a parse error never depends on a value.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use toml::Value;

use crate::error::{Error, names};
use crate::model::color::HexColor;
use crate::model::ids::{Appearance, IdProblem, Origin, ThemeId};
use crate::model::palette::{AnsiSet, Palette};

/// A stage in a theme's life; see [`Parsed`] and [`Validated`].
pub trait Phase {
    /// The data the stage carries.
    type Data;
}

/// The phase right after parsing: values are typed, but not yet interpreted.
#[derive(Debug)]
pub struct Parsed;

impl Phase for Parsed {
    type Data = RawTheme;
}

/// The phase after validation: every value is checked and interpreted.
#[derive(Debug)]
pub struct Validated;

impl Phase for Validated {
    type Data = ThemeData;
}

/// A theme document, parameterised by how far it has been processed.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct Theme<P: Phase> {
    pub id: ThemeId,
    pub name: String,
    pub appearance: Appearance,
    pub derived: bool,
    pub origin: Origin,
    pub path: PathBuf,
    pub data: P::Data,
}

/// Exactly what the TOML said, keyed and typed but not yet interpreted.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct RawTheme {
    pub palette: BTreeMap<String, Value>,
    pub ansi: BTreeMap<String, Value>,
    pub targets: BTreeMap<String, BTreeMap<String, Value>>,
}

/// Interpreted values; only renderers read this.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct ThemeData {
    pub palette: Palette,
    pub ansi: AnsiSet,
    pub wt: WtOverrides,
    pub omp: BTreeMap<String, OverrideValue>,
    pub herdr: BTreeMap<String, OverrideValue>,
}

/// The four Windows Terminal scheme colours a theme may override.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WtOverrides {
    /// `background`, defaulting to `base00`.
    pub background: Option<HexColor>,
    /// `foreground`, defaulting to `base05`.
    pub foreground: Option<HexColor>,
    /// `cursorColor`, defaulting to `base0D`.
    pub cursor_color: Option<HexColor>,
    /// `selectionBackground`, defaulting to `base02`.
    pub selection_background: Option<HexColor>,
}

/// A `[targets.*]` value; base16 cannot express these.
///
/// There is no parser method: [`validate`](crate::validate) builds these three cases
/// directly from the raw [`Value`], because the failure message differs per case.
#[allow(missing_docs)]
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OverrideValue {
    Color(HexColor),
    Index(u8),
    Text(String),
}

/// The root keys a theme file may declare.
const ROOT_KEYS: [&str; 7] = [
    "id",
    "name",
    "appearance",
    "derived",
    "palette",
    "ansi",
    "targets",
];

impl Theme<Parsed> {
    /// Parses `source` as a theme file.
    ///
    /// IO-free: the caller supplies the file's bytes and the path used in diagnostics.
    pub fn parse(path: PathBuf, source: &str, origin: Origin) -> Result<Self, Error> {
        let table = source
            .parse::<toml::Table>()
            .map_err(|source| Error::ThemeUnparseable {
                path: path.clone(),
                source,
            })?;

        if let Some(key) = table.keys().find(|key| !ROOT_KEYS.contains(&key.as_str())) {
            return Err(Error::UnknownKey {
                path,
                section: "the theme file",
                key: key.clone(),
                accepted: names(&ROOT_KEYS),
            });
        }

        let raw_id = string_field(&table, "id", &path)?;
        let id = ThemeId::parse(raw_id).map_err(|problem| match problem {
            IdProblem::Empty => Error::ThemeIdEmpty { path: path.clone() },
            IdProblem::BadChar(ch) => Error::ThemeIdInvalidChar {
                path: path.clone(),
                id: raw_id.to_owned(),
                ch,
            },
        })?;
        let name = string_field(&table, "name", &path)?.to_owned();
        let appearance = match string_field(&table, "appearance", &path)? {
            "dark" => Appearance::Dark,
            "light" => Appearance::Light,
            value => {
                return Err(Error::UnknownAppearance {
                    path,
                    value: value.to_owned(),
                });
            }
        };
        let derived = match table.get("derived") {
            None => false,
            Some(Value::Boolean(value)) => *value,
            Some(other) => return Err(field_type(&path, "derived", "a boolean", other)),
        };
        let palette = match table.get("palette") {
            None => {
                return Err(Error::MissingField {
                    path,
                    key: "palette",
                });
            }
            Some(Value::Table(table)) => to_map(table),
            Some(other) => return Err(field_type(&path, "palette", "a table", other)),
        };
        let ansi = match table.get("ansi") {
            None => BTreeMap::new(),
            Some(Value::Table(table)) => to_map(table),
            Some(other) => return Err(field_type(&path, "ansi", "a table", other)),
        };
        let mut targets = BTreeMap::new();
        match table.get("targets") {
            None => {}
            Some(Value::Table(sections)) => {
                for (section, value) in sections {
                    match value {
                        Value::Table(keys) => {
                            targets.insert(section.clone(), to_map(keys));
                        }
                        other => {
                            return Err(field_type(&path, "targets", "a table of tables", other));
                        }
                    }
                }
            }
            Some(other) => return Err(field_type(&path, "targets", "a table", other)),
        }

        Ok(Self {
            id,
            name,
            appearance,
            derived,
            origin,
            path,
            data: RawTheme {
                palette,
                ansi,
                targets,
            },
        })
    }
}

impl<P: Phase> fmt::Display for Theme<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

fn string_field<'a>(
    table: &'a toml::Table,
    key: &'static str,
    path: &Path,
) -> Result<&'a str, Error> {
    match table.get(key) {
        None => Err(Error::MissingField {
            path: path.to_path_buf(),
            key,
        }),
        Some(Value::String(value)) => Ok(value),
        Some(other) => Err(field_type(path, key, "a string", other)),
    }
}

fn field_type(path: &Path, key: &'static str, expected: &'static str, found: &Value) -> Error {
    Error::FieldType {
        path: path.to_path_buf(),
        key,
        expected,
        found: type_name(found),
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "a string",
        Value::Integer(_) => "an integer",
        Value::Float(_) => "a float",
        Value::Boolean(_) => "a boolean",
        Value::Datetime(_) => "a datetime",
        Value::Array(_) => "an array",
        Value::Table(_) => "a table",
    }
}

/// Copies a TOML table into a map ordered by key; the order decides which of two
/// spellings of one base16 entry is reported during validation.
fn to_map(table: &toml::Table) -> BTreeMap<String, Value> {
    table
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Theme;
    use crate::error::Error;
    use crate::model::ids::{Appearance, Origin};

    /// A minimal document that parses and validates.
    const VALID: &str = include_str!("../../tests/fixtures/themes/overrides.toml");

    #[test]
    fn parse_reads_the_document_shape() {
        let theme = Theme::parse(
            PathBuf::from("tests/fixtures/themes/overrides.toml"),
            VALID,
            Origin::User,
        )
        .unwrap();
        assert_eq!(theme.id.as_str(), "overrides");
        assert_eq!(theme.name, "Overrides");
        assert_eq!(theme.appearance, Appearance::Dark);
        assert!(!theme.derived);
        assert_eq!(theme.origin, Origin::User);
        assert_eq!(theme.data.palette.len(), 16);
        assert_eq!(theme.data.ansi.len(), 1);
        assert_eq!(theme.data.targets.len(), 3);
    }

    #[test]
    fn unparseable_toml_names_the_file() {
        let error = Theme::parse(PathBuf::from("theme.toml"), "id = ", Origin::User).unwrap_err();
        assert!(matches!(error, Error::ThemeUnparseable { .. }));
        assert!(error.to_string().contains("theme.toml"));
    }

    #[test]
    fn unknown_root_key_lists_the_accepted_keys() {
        let error = Theme::parse(PathBuf::from("theme.toml"), "foo = 1", Origin::User).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("unknown key `foo` in the theme file"),
            "{message}"
        );
        assert!(
            message.contains(
                "accepted keys are id, name, appearance, derived, palette, ansi, targets"
            ),
            "{message}"
        );
    }

    #[test]
    fn missing_and_mistyped_keys_name_the_key() {
        let source = VALID.replace("name = \"Overrides\"", "");
        let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();
        assert!(
            matches!(error, Error::MissingField { key: "name", .. }),
            "{error}"
        );

        let source = VALID.replace("name = \"Overrides\"", "name = 42");
        let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("`name` must be a string"), "{message}");
        assert!(message.contains("found an integer"), "{message}");
    }

    #[test]
    fn unknown_appearance_names_the_value() {
        let source = VALID.replace("appearance = \"dark\"", "appearance = \"dim\"");
        let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();
        assert!(error.to_string().contains("\"dim\""), "{error}");
    }

    #[test]
    fn an_invalid_theme_id_reports_the_offending_character() {
        let source = VALID.replace("id = \"overrides\"", "id = \"Overrides\"");
        let error = Theme::parse(PathBuf::from("theme.toml"), &source, Origin::User).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("theme id `Overrides` contains `O`"),
            "{message}"
        );
    }
}
