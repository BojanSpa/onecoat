use std::fmt;
use std::path::Path;

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstInputValue, CstNode, CstObject, CstObjectProp, CstRootNode};
use jsonc_parser::errors::ParseError;
use serde_json::Value;

use crate::Error;
use crate::model::ids::Slot;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Key(Vec<String>);

impl Key {
    pub fn parse(dotted: &str) -> Self {
        Self(dotted.split('.').map(str::to_owned).collect())
    }

    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn segments(&self) -> &[String] {
        &self.0
    }

    fn prefix(&self, length: usize) -> Self {
        Self(self.0[..length].to_vec())
    }

    pub(crate) fn join(&self, segment: &str) -> Self {
        let mut segments = self.0.clone();
        segments.push(segment.to_owned());
        Self(segments)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("the document root");
        }

        f.write_str(&self.0.join("."))
    }
}

pub const NEW_DOCUMENT: &str = "{}\n";

#[derive(Clone, PartialEq, Debug)]
pub enum Edit {
    Set {
        key: Key,
        value: Value,
    },
    Pair {
        key: Key,
        side: Slot,
        name: String,
        fill: Fill,
    },
    Element {
        key: Key,
        name: String,
        value: Value,
    },
    Repoint {
        key: Key,
        field: String,
        side: Slot,
        name: String,
        fill: Fill,
    },
}

impl Edit {
    pub fn key(&self) -> &Key {
        match self {
            Self::Set { key, .. }
            | Self::Pair { key, .. }
            | Self::Element { key, .. }
            | Self::Repoint { key, .. } => key,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pin {
    pub name: String,
    pub scheme: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fill {
    FromString,
    BuiltIn,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Splice {
    pub text: String,
    pub changed: Vec<Key>,
}

pub fn splice(path: &Path, source: &str, edits: &[Edit]) -> Result<Splice, Error> {
    let root = parse(path, source)?;

    let Some(document) = root.object_value() else {
        return Err(not_an_object(path, &Key::root()));
    };

    let mut changed = Vec::new();
    for edit in edits {
        let before = root.to_serde_value();
        apply(path, &document, edit)?;

        if root.to_serde_value() != before {
            changed.push(edit.key().clone());
        }
    }

    Ok(Splice {
        text: root.to_string(),
        changed,
    })
}

pub fn verify(path: &Path, source: &str) -> Result<(), Error> {
    parse(path, source).map(|_| ())
}

pub fn value(path: &Path, source: &str, key: &Key) -> Result<Option<Value>, Error> {
    let root = parse(path, source)?;

    let Some(document) = root.object_value() else {
        return Err(not_an_object(path, &Key::root()));
    };

    let Some((first, rest)) = key.segments().split_first() else {
        return Ok(root.to_serde_value());
    };

    let mut node = document.get(first).and_then(|property| property.value());
    for segment in rest {
        node = node.and_then(|node| descend(node, segment));
    }

    Ok(node.and_then(|node| node.to_serde_value()))
}

fn descend(node: CstNode, name: &str) -> Option<CstNode> {
    match node.as_object() {
        Some(object) => object.get(name)?.value(),
        None => node
            .as_array()?
            .elements()
            .into_iter()
            .find(|element| element_name(element).as_deref() == Some(name)),
    }
}

fn parse(path: &Path, source: &str) -> Result<CstRootNode, Error> {
    CstRootNode::parse(source, &ParseOptions::default()).map_err(|error| unparseable(path, &error))
}

fn unparseable(path: &Path, error: &ParseError) -> Error {
    Error::JsoncUnparseable {
        path: path.to_path_buf(),
        line: error.line_display(),
        column: error.column_display(),
        message: error.kind().to_string(),
    }
}

fn not_an_object(path: &Path, key: &Key) -> Error {
    Error::KeyNotLocatable {
        path: path.to_path_buf(),
        key: key.clone(),
        expected: "a JSON object",
    }
}

fn not_an_array(path: &Path, key: &Key) -> Error {
    Error::KeyNotLocatable {
        path: path.to_path_buf(),
        key: key.clone(),
        expected: "a JSON array",
    }
}

fn apply(path: &Path, document: &CstObject, edit: &Edit) -> Result<(), Error> {
    match edit {
        Edit::Set { key, value } => set(path, document, key, input(value)),
        Edit::Pair {
            key,
            side,
            name,
            fill,
        } => set_pair(path, document, key, *side, name, *fill),
        Edit::Element { key, name, value } => set_element(path, document, key, name, input(value)),
        Edit::Repoint {
            key,
            field,
            side,
            name,
            fill,
        } => set_repoint(path, document, key, field, *side, name, *fill),
    }
}

fn set(path: &Path, document: &CstObject, key: &Key, value: CstInputValue) -> Result<(), Error> {
    let (object, name) = holder(path, document, key)?;
    put(&object, name, value);
    Ok(())
}

fn set_pair(
    path: &Path,
    document: &CstObject,
    key: &Key,
    side: Slot,
    name: &str,
    fill: Fill,
) -> Result<(), Error> {
    let (object, leaf) = holder(path, document, key)?;

    let Some(property) = object.get(leaf) else {
        object.append(leaf, pair(side, name, None));
        return Ok(());
    };

    let Some(value) = property.value() else {
        property.set_value(pair(side, name, None));
        return Ok(());
    };

    if value.as_object().is_some() {
        let child = key.join(side.name());

        return set(
            path,
            document,
            &child,
            CstInputValue::String(name.to_owned()),
        );
    }

    let previous = value
        .as_string_lit()
        .and_then(|literal| literal.decoded_value().ok());

    match (previous, fill) {
        (Some(previous), Fill::FromString) => property.set_value(pair(side, name, Some(&previous))),
        (Some(_), Fill::BuiltIn) => property.set_value(pair(side, name, None)),
        (None, _) => return Err(not_a_pair(path, key)),
    }

    Ok(())
}

fn set_element(
    path: &Path,
    document: &CstObject,
    key: &Key,
    name: &str,
    value: CstInputValue,
) -> Result<(), Error> {
    let (object, leaf) = holder(path, document, key)?;

    let Some(property) = object.get(leaf) else {
        object.append(leaf, CstInputValue::Array(vec![value]));
        return Ok(());
    };

    let Some(array) = property.array_value() else {
        return Err(not_an_array(path, key));
    };

    let elements = array.elements();

    let found = elements
        .iter()
        .position(|element| element_name(element).as_deref() == Some(name));

    match found {
        Some(index) => {
            elements[index].clone().remove();
            array.insert(index, value);
        }
        None => {
            array.append(value);
        }
    }

    Ok(())
}

fn set_repoint(
    path: &Path,
    document: &CstObject,
    key: &Key,
    field: &str,
    side: Slot,
    name: &str,
    fill: Fill,
) -> Result<(), Error> {
    let Some(location) = locate(document, key) else {
        return Ok(());
    };

    let Some(array) = location.array_value() else {
        return Err(not_an_array(path, key));
    };

    for element in array.elements() {
        let Some(object) = element.as_object() else {
            continue;
        };

        let Some(property) = object.get(field) else {
            continue;
        };

        let Some(value) = property.value() else {
            property.set_value(pair(side, name, None));
            continue;
        };

        if let Some(inner) = value.as_object() {
            put(&inner, side.name(), CstInputValue::String(name.to_owned()));
            continue;
        }

        let Some(previous) = value
            .as_string_lit()
            .and_then(|literal| literal.decoded_value().ok())
        else {
            return Err(not_a_pair(path, key));
        };

        let repointed = match fill {
            Fill::FromString => pair(side, name, Some(previous.as_str())),
            Fill::BuiltIn => pair(side, name, None),
        };

        property.set_value(repointed);
    }

    Ok(())
}

pub fn pinned(path: &Path, source: &str, key: &Key, field: &str) -> Result<Vec<Pin>, Error> {
    let root = parse(path, source)?;

    let Some(document) = root.object_value() else {
        return Err(not_an_object(path, &Key::root()));
    };

    let Some(location) = locate(&document, key) else {
        return Ok(Vec::new());
    };

    let Some(array) = location.array_value() else {
        return Err(not_an_array(path, key));
    };

    let mut pins = Vec::new();
    for (index, element) in array.elements().iter().enumerate() {
        let Some(object) = element.as_object() else {
            continue;
        };

        let Some(value) = object.get(field).and_then(|property| property.value()) else {
            continue;
        };

        pins.push(Pin {
            name: profile_label(&object, index),
            scheme: pinned_scheme(path, key, &value)?,
        });
    }

    Ok(pins)
}

fn locate(document: &CstObject, key: &Key) -> Option<CstObjectProp> {
    let (leaf, owners) = key.segments().split_last()?;
    let mut object = document.clone();
    for owner in owners {
        object = object.get(owner)?.object_value()?;
    }

    object.get(leaf)
}

fn put(object: &CstObject, name: &str, value: CstInputValue) {
    match object.get(name) {
        Some(property) => property.set_value(value),
        None => {
            object.append(name, value);
        }
    }
}

fn profile_label(object: &CstObject, index: usize) -> String {
    for name in ["name", "guid"] {
        let label = object
            .get(name)
            .and_then(|property| property.value())
            .and_then(|value| {
                value
                    .as_string_lit()
                    .and_then(|lit| lit.decoded_value().ok())
            });

        if let Some(label) = label {
            return label;
        }
    }

    format!("#{index}")
}

fn pinned_scheme(path: &Path, key: &Key, value: &CstNode) -> Result<String, Error> {
    if let Some(literal) = value.as_string_lit()
        && let Ok(text) = literal.decoded_value()
    {
        return Ok(text);
    }

    if let Some(object) = value.as_object() {
        let sides: Vec<String> = [Slot::Dark, Slot::Light]
            .into_iter()
            .filter_map(|side| {
                object
                    .get(side.name())
                    .and_then(|property| property.value())
                    .and_then(|value| {
                        value
                            .as_string_lit()
                            .and_then(|lit| lit.decoded_value().ok())
                    })
            })
            .collect();

        if !sides.is_empty() {
            return Ok(sides.join("/"));
        }
    }

    Err(not_a_pair(path, key))
}

fn pair(side: Slot, name: &str, previous: Option<&str>) -> CstInputValue {
    let mut entries = vec![(
        side.name().to_owned(),
        CstInputValue::String(name.to_owned()),
    )];

    if let Some(previous) = previous {
        entries.push((
            other(side).name().to_owned(),
            CstInputValue::String(previous.to_owned()),
        ));
    }

    CstInputValue::Object(entries)
}

fn other(side: Slot) -> Slot {
    match side {
        Slot::Dark => Slot::Light,
        Slot::Light => Slot::Dark,
    }
}

fn not_a_pair(path: &Path, key: &Key) -> Error {
    Error::KeyNotLocatable {
        path: path.to_path_buf(),
        key: key.clone(),
        expected: "a JSON object or a string",
    }
}

fn holder<'k>(
    path: &Path,
    document: &CstObject,
    key: &'k Key,
) -> Result<(CstObject, &'k str), Error> {
    let Some((leaf, owners)) = key.segments().split_last() else {
        return Err(not_an_object(path, key));
    };

    let mut object = document.clone();
    for (index, owner) in owners.iter().enumerate() {
        object = match object.get(owner) {
            None => object
                .append(owner, CstInputValue::Object(Vec::new()))
                .object_value()
                .ok_or_else(|| not_an_object(path, &key.prefix(index + 1)))?,
            Some(property) => property
                .object_value()
                .ok_or_else(|| not_an_object(path, &key.prefix(index + 1)))?,
        };
    }

    Ok((object, leaf))
}

fn element_name(element: &CstNode) -> Option<String> {
    element
        .as_object()?
        .get("name")?
        .value()?
        .as_string_lit()?
        .decoded_value()
        .ok()
}

fn input(value: &Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(flag) => CstInputValue::Bool(*flag),
        Value::Number(number) => CstInputValue::Number(number.to_string()),
        Value::String(text) => CstInputValue::String(text.clone()),
        Value::Array(items) => CstInputValue::Array(items.iter().map(input).collect()),
        Value::Object(map) => CstInputValue::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), input(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
#[path = "../tests/unit/jsonc.rs"]
mod tests;
