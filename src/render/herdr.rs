use std::path::Path;

use toml_edit::{Decor, DocumentMut, Item, Table, Value as TomlValue};

use crate::Error;
use crate::jsonc::{Key, Splice};
use crate::model::color::HexColor;
use crate::model::ids::Slot;
use crate::model::palette::Base16Entry;
use crate::model::theme::{OverrideValue, Theme, Validated};

const THEME_KEY: &str = "theme";
const CUSTOM_KEY: &str = "custom";
const NAME_KEY: &str = "name";
const AUTO_SWITCH_KEY: &str = "auto_switch";
const DARK_NAME_KEY: &str = "dark_name";
const LIGHT_NAME_KEY: &str = "light_name";
const BASE_THEME: &str = "terminal";
const TOKEN_HINT: &str = "the token names herdr defines, such as `panel_bg`";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    Shared,
    Appearance,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HerdrToken {
    Accent,
    ActiveRowBg,
    Blue,
    Green,
    PanelBg,
    Red,
    SelectionBg,
    SidebarBg,
    Text,
    Yellow,
}

impl HerdrToken {
    pub const ALL: [Self; 10] = [
        Self::Accent,
        Self::ActiveRowBg,
        Self::Blue,
        Self::Green,
        Self::PanelBg,
        Self::Red,
        Self::SelectionBg,
        Self::SidebarBg,
        Self::Text,
        Self::Yellow,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Accent => "accent",
            Self::ActiveRowBg => "active_row_bg",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::PanelBg => "panel_bg",
            Self::Red => "red",
            Self::SelectionBg => "selection_bg",
            Self::SidebarBg => "sidebar_bg",
            Self::Text => "text",
            Self::Yellow => "yellow",
        }
    }

    pub fn from_name(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|token| token.name() == raw)
    }

    fn role(self) -> Base16Entry {
        match self {
            Self::Accent | Self::Blue => Base16Entry::B0D,
            Self::ActiveRowBg | Self::SelectionBg => Base16Entry::B02,
            Self::Green => Base16Entry::B0B,
            Self::PanelBg => Base16Entry::B00,
            Self::Red => Base16Entry::B08,
            Self::SidebarBg => Base16Entry::B01,
            Self::Text => Base16Entry::B05,
            Self::Yellow => Base16Entry::B0A,
        }
    }

    fn scope(self) -> Scope {
        match self {
            Self::ActiveRowBg
            | Self::PanelBg
            | Self::SelectionBg
            | Self::SidebarBg
            | Self::Text => Scope::Appearance,
            Self::Accent | Self::Blue | Self::Green | Self::Red | Self::Yellow => Scope::Shared,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Value {
    Color(HexColor),
    Bool(bool),
    Text(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Edit {
    pub path: Vec<String>,
    pub value: Value,
}

impl Edit {
    fn key(&self) -> Key {
        Key::parse(&self.path.join("."))
    }

    fn to_toml(&self) -> TomlValue {
        match &self.value {
            Value::Color(color) => TomlValue::from(color.to_string()),
            Value::Bool(value) => TomlValue::from(*value),
            Value::Text(text) => TomlValue::from(text.clone()),
        }
    }
}

pub fn edits(theme: &Theme<Validated>, slot: Slot) -> Result<Vec<Edit>, Error> {
    reject_unknown_keys(theme)?;

    let name = name(theme)?;

    let mut edits = vec![
        Edit {
            path: path(&[THEME_KEY, NAME_KEY]),
            value: Value::Text(name.clone()),
        },
        Edit {
            path: path(&[THEME_KEY, AUTO_SWITCH_KEY]),
            value: Value::Bool(true),
        },
        Edit {
            path: path(&[THEME_KEY, DARK_NAME_KEY]),
            value: Value::Text(name.clone()),
        },
        Edit {
            path: path(&[THEME_KEY, LIGHT_NAME_KEY]),
            value: Value::Text(name),
        },
    ];

    for token in HerdrToken::ALL {
        let mut path = match token.scope() {
            Scope::Shared => path(&[THEME_KEY, CUSTOM_KEY]),
            Scope::Appearance => path(&[THEME_KEY, CUSTOM_KEY, slot.name()]),
        };

        path.push(token.name().to_owned());

        edits.push(Edit {
            path,
            value: Value::Color(color(theme, token)?),
        });
    }

    Ok(edits)
}

pub fn parse(path: &Path, source: &str) -> Result<DocumentMut, Error> {
    source
        .parse::<DocumentMut>()
        .map_err(|source| Error::TomlUnparseable {
            path: path.to_path_buf(),
            source: Box::new(source),
        })
}

pub fn value(document: &DocumentMut, path: &[String]) -> Option<serde_json::Value> {
    let (leaf, owners) = path.split_last()?;

    let mut table = document.as_table();
    for owner in owners {
        table = table.get(owner)?.as_table()?;
    }

    table.get(leaf).map(to_json)
}

pub fn is_shared(edit: &Edit) -> bool {
    !matches!(
        edit.path.as_slice(),
        [theme, custom, side, ..]
            if theme == THEME_KEY && custom == CUSTOM_KEY && is_slot(side)
    )
}

fn is_slot(name: &str) -> bool {
    [Slot::Dark, Slot::Light]
        .into_iter()
        .any(|slot| slot.name() == name)
}

pub fn splice(path: &Path, source: &str, edits: &[Edit]) -> Result<Splice, Error> {
    let mut document = parse(path, source)?;

    let mut changed = Vec::new();
    for edit in edits {
        if set(path, &mut document, edit)? {
            changed.push(edit.key());
        }
    }

    let text = end_of_lines(source, &document.to_string());

    Ok(Splice { text, changed })
}

fn set(path: &Path, document: &mut DocumentMut, edit: &Edit) -> Result<bool, Error> {
    let Some((key, tables)) = edit.path.split_last() else {
        return Ok(false);
    };

    let mut current = document.as_table_mut();
    for name in tables {
        if !current.contains_key(name) {
            let mut table = Table::new();
            table.set_implicit(false);
            current.insert(name, Item::Table(table));
        }

        let not_a_table = || Error::KeyNotLocatable {
            path: path.to_path_buf(),
            key: edit.key(),
            expected: "a table holding the theme keys",
        };

        current = match current.get_mut(name) {
            Some(Item::Table(table)) => table,
            Some(_) => return Err(not_a_table()),
            None => return Err(not_a_table()),
        };
    }

    let Some(item) = current.get_mut(key) else {
        current.insert(key, Item::Value(edit.to_toml()));

        return Ok(true);
    };

    let not_a_value = || Error::KeyNotLocatable {
        path: path.to_path_buf(),
        key: edit.key(),
        expected: "a value onecoat can replace",
    };

    let Some(existing) = item.as_value_mut() else {
        return Err(not_a_value());
    };

    let mut replacement = edit.to_toml();
    let mut plain = existing.clone();
    *plain.decor_mut() = Decor::default();

    if plain.to_string() == replacement.to_string() {
        return Ok(false);
    }

    *replacement.decor_mut() = existing.decor().clone();
    *existing = replacement;

    Ok(true)
}

fn name(theme: &Theme<Validated>) -> Result<String, Error> {
    match theme.data.herdr.get(NAME_KEY) {
        None => Ok(BASE_THEME.to_owned()),
        Some(OverrideValue::Text(text)) => Ok(text.clone()),
        Some(OverrideValue::Color(_)) => Err(Error::FieldType {
            path: theme.path.clone(),
            key: "targets.herdr.name",
            expected: "a string",
            found: "a colour",
        }),
        Some(OverrideValue::Index(_)) => Err(Error::FieldType {
            path: theme.path.clone(),
            key: "targets.herdr.name",
            expected: "a string",
            found: "an integer",
        }),
    }
}

fn reject_unknown_keys(theme: &Theme<Validated>) -> Result<(), Error> {
    match theme
        .data
        .herdr
        .keys()
        .find(|key| key.as_str() != NAME_KEY && HerdrToken::from_name(key).is_none())
    {
        Some(key) => Err(Error::UnknownKey {
            path: theme.path.clone(),
            section: "[targets.herdr]",
            key: key.clone(),
            accepted: TOKEN_HINT.to_owned(),
        }),
        None => Ok(()),
    }
}

fn color(theme: &Theme<Validated>, token: HerdrToken) -> Result<HexColor, Error> {
    match theme.data.herdr.get(token.name()) {
        Some(OverrideValue::Color(color)) => Ok(*color),
        Some(OverrideValue::Text(text)) => Err(Error::MalformedColor {
            path: theme.path.clone(),
            key: format!("targets.herdr.{}", token.name()),
            value: text.clone(),
        }),
        Some(OverrideValue::Index(index)) => Err(Error::MalformedColor {
            path: theme.path.clone(),
            key: format!("targets.herdr.{}", token.name()),
            value: index.to_string(),
        }),
        None => Ok(theme.data.palette[token.role()]),
    }
}

fn path(segments: &[&str]) -> Vec<String> {
    segments
        .iter()
        .map(|segment| (*segment).to_owned())
        .collect()
}

fn end_of_lines(source: &str, rendered: &str) -> String {
    if !source.contains("\r\n") {
        return rendered.to_owned();
    }

    rendered.replace("\r\n", "\n").replace('\n', "\r\n")
}

fn to_json(item: &Item) -> serde_json::Value {
    match item {
        Item::Value(value) => scalar_json(value),
        Item::Table(table) => table_json(table),
        Item::ArrayOfTables(array) => {
            serde_json::Value::Array(array.iter().map(table_json).collect())
        }
        Item::None => serde_json::Value::Null,
    }
}

fn table_json(table: &Table) -> serde_json::Value {
    serde_json::Value::Object(
        table
            .iter()
            .map(|(name, item)| (name.to_owned(), to_json(item)))
            .collect(),
    )
}

fn scalar_json(value: &TomlValue) -> serde_json::Value {
    match value {
        TomlValue::String(text) => serde_json::Value::String(text.value().clone()),
        TomlValue::Integer(number) => serde_json::Value::Number((*number.value()).into()),
        TomlValue::Float(number) => serde_json::Number::from_f64(*number.value())
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        TomlValue::Boolean(flag) => serde_json::Value::Bool(*flag.value()),
        TomlValue::Datetime(datetime) => serde_json::Value::String(datetime.value().to_string()),
        TomlValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(scalar_json).collect())
        }
        TomlValue::InlineTable(table) => serde_json::Value::Object(
            table
                .iter()
                .map(|(name, value)| (name.to_owned(), scalar_json(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/render/herdr.rs"]
mod tests;
