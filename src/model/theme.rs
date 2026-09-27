use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use toml::Value;

use crate::error::{Error, accepted_keys};
use crate::model::color::HexColor;
use crate::model::ids::{Appearance, IdProblem, Origin, ThemeId};
use crate::model::palette::{AnsiSet, Palette};

pub trait Phase {
    type Data;
}

#[derive(Debug)]
pub struct Parsed;

impl Phase for Parsed {
    type Data = RawTheme;
}

#[derive(Debug)]
pub struct Validated;

impl Phase for Validated {
    type Data = ThemeData;
}

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

#[derive(Debug)]
pub struct RawTheme {
    pub palette: BTreeMap<String, Value>,
    pub ansi: BTreeMap<String, Value>,
    pub targets: BTreeMap<String, BTreeMap<String, Value>>,
}

#[derive(Debug)]
pub struct ThemeData {
    pub palette: Palette,
    pub ansi: AnsiSet,
    pub wt: WtOverrides,
    pub omp: BTreeMap<String, OverrideValue>,
    pub herdr: BTreeMap<String, OverrideValue>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WtOverrides {
    pub background: Option<HexColor>,
    pub foreground: Option<HexColor>,
    pub cursor_color: Option<HexColor>,
    pub selection_background: Option<HexColor>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OverrideValue {
    Color(HexColor),
    Index(u8),
    Text(String),
}

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
                accepted: accepted_keys(&ROOT_KEYS),
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

fn to_map(table: &toml::Table) -> BTreeMap<String, Value> {
    table
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/model/theme.rs"]
mod tests;
