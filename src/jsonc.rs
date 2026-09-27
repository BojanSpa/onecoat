use std::fmt;
use std::path::Path;

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstInputValue, CstNode, CstObject, CstRootNode};
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

    fn segments(&self) -> &[String] {
        &self.0
    }

    fn prefix(&self, length: usize) -> Self {
        Self(self.0[..length].to_vec())
    }

    fn join(&self, segment: &str) -> Self {
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
}

impl Edit {
    pub fn key(&self) -> &Key {
        match self {
            Self::Set { key, .. } | Self::Pair { key, .. } | Self::Element { key, .. } => key,
        }
    }
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
    }
}

fn set(path: &Path, document: &CstObject, key: &Key, value: CstInputValue) -> Result<(), Error> {
    let (object, name) = holder(path, document, key)?;
    match object.get(name) {
        Some(property) => property.set_value(value),
        None => {
            object.append(name, value);
        }
    }
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
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::{Edit, Fill, Key, NEW_DOCUMENT, Splice, splice, verify};
    use crate::Error;
    use crate::model::ids::Slot;

    fn key(dotted: &str) -> Key {
        Key::parse(dotted)
    }

    fn splice_ok(source: &str, edits: &[Edit]) -> Splice {
        splice(Path::new("settings.json"), source, edits).unwrap()
    }

    #[test]
    fn a_missing_key_is_inserted_after_the_last_one() {
        let source = "{\n    \"a\": 1\n}";
        let spliced = splice_ok(
            source,
            &[Edit::Set {
                key: key("theme"),
                value: json!({ "dark": "onecoat-dark" }),
            }],
        );

        assert_eq!(
            spliced.text,
            "{\n    \"a\": 1,\n    \"theme\": {\n        \"dark\": \"onecoat-dark\"\n    }\n}"
        );
        assert_eq!(spliced.changed, [key("theme")]);
    }

    #[test]
    fn missing_intermediates_are_created() {
        let spliced = splice_ok(
            "{}\n",
            &[Edit::Pair {
                key: key("profiles.defaults.colorScheme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::FromString,
            }],
        );

        assert_eq!(
            spliced.text,
            "{\n  \"profiles\": {\n    \"defaults\": {\n      \"colorScheme\": {\n        \"dark\": \"onecoat-dark\"\n      }\n    }\n  }\n}\n"
        );
        assert_eq!(spliced.changed, [key("profiles.defaults.colorScheme")]);
    }

    #[test]
    fn a_pair_side_leaves_the_other_appearance_alone() {
        let source = "{\"theme\": {\"light\": \"light\", \"dark\": \"dark\"}}";
        let spliced = splice_ok(
            source,
            &[Edit::Pair {
                key: key("theme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::BuiltIn,
            }],
        );

        assert_eq!(
            spliced.text,
            "{\"theme\": {\"light\": \"light\", \"dark\": \"onecoat-dark\"}}"
        );
        assert_eq!(spliced.changed, [key("theme")]);
    }

    #[test]
    fn a_pair_carries_a_bare_string_into_the_other_side_when_asked() {
        let with_string = "{\n  \"theme\": \"dark\"\n}\n";
        let carried = splice_ok(
            with_string,
            &[Edit::Pair {
                key: key("theme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::FromString,
            }],
        );
        assert_eq!(
            carried.text,
            "{\n  \"theme\": {\n    \"dark\": \"onecoat-dark\",\n    \"light\": \"dark\"\n  }\n}\n"
        );

        let dropped = splice_ok(
            with_string,
            &[Edit::Pair {
                key: key("theme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::BuiltIn,
            }],
        );
        assert_eq!(
            dropped.text,
            "{\n  \"theme\": {\n    \"dark\": \"onecoat-dark\"\n  }\n}\n"
        );
    }

    #[test]
    fn an_array_element_is_replaced_in_place_and_foreign_elements_survive() {
        let source = "{\n  \"themes\": [\n    {\n      \"name\": \"mine\",\n      \"frame\": \"#000000\"\n    },\n    {\n      \"name\": \"onecoat-dark\",\n      \"frame\": \"#111111\"\n    }\n  ]\n}\n";
        let spliced = splice_ok(
            source,
            &[Edit::Element {
                key: key("themes"),
                name: "onecoat-dark".to_owned(),
                value: json!({ "name": "onecoat-dark", "frame": "#0b1018" }),
            }],
        );

        assert_eq!(
            spliced.text,
            "{\n  \"themes\": [\n    {\n      \"name\": \"mine\",\n      \"frame\": \"#000000\"\n    },\n    {\n      \"frame\": \"#0b1018\",\n      \"name\": \"onecoat-dark\"\n    }\n  ]\n}\n"
        );
        assert_eq!(spliced.changed, [key("themes")]);
    }

    #[test]
    fn an_array_element_is_appended_when_its_name_is_absent() {
        let spliced = splice_ok(
            "{\n  \"themes\": [\n    {\n      \"name\": \"mine\"\n    }\n  ]\n}\n",
            &[Edit::Element {
                key: key("themes"),
                name: "onecoat-light".to_owned(),
                value: json!({ "name": "onecoat-light" }),
            }],
        );

        assert_eq!(
            spliced.text,
            "{\n  \"themes\": [\n    {\n      \"name\": \"mine\"\n    },\n    {\n      \"name\": \"onecoat-light\"\n    }\n  ]\n}\n"
        );
    }

    #[test]
    fn an_absent_array_holds_the_element_alone() {
        let spliced = splice_ok(
            "{}\n",
            &[Edit::Element {
                key: key("schemes"),
                name: "onecoat-dark".to_owned(),
                value: json!({ "name": "onecoat-dark" }),
            }],
        );

        assert_eq!(
            spliced.text,
            "{\n  \"schemes\": [\n    {\n      \"name\": \"onecoat-dark\"\n    }\n  ]\n}\n"
        );
    }

    #[test]
    fn a_created_document_keeps_the_seed_around_the_new_key() {
        let spliced = splice_ok(
            NEW_DOCUMENT,
            &[Edit::Element {
                key: key("schemes"),
                name: "onecoat-dark".to_owned(),
                value: json!({ "name": "onecoat-dark" }),
            }],
        );

        assert_eq!(
            spliced.text,
            "{\n  \"schemes\": [\n    {\n      \"name\": \"onecoat-dark\"\n    }\n  ]\n}\n"
        );
    }

    #[test]
    fn comments_line_endings_and_indentation_survive() {
        let source = "{\r\n  // the theme\r\n  \"a\": 1,\r\n  \"theme\": \"dark\"\r\n}";
        let spliced = splice_ok(
            source,
            &[Edit::Pair {
                key: key("theme"),
                side: Slot::Light,
                name: "onecoat-light".to_owned(),
                fill: Fill::FromString,
            }],
        );

        assert_eq!(
            spliced.text,
            "{\r\n  // the theme\r\n  \"a\": 1,\r\n  \"theme\": {\r\n    \"light\": \"onecoat-light\",\r\n    \"dark\": \"dark\"\r\n  }\r\n}"
        );
    }

    #[test]
    fn a_second_identical_splice_changes_nothing() {
        let edits = [
            Edit::Pair {
                key: key("theme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::FromString,
            },
            Edit::Element {
                key: key("themes"),
                name: "onecoat-dark".to_owned(),
                value: json!({ "name": "onecoat-dark" }),
            },
        ];
        let once = splice_ok("{\n  \"themes\": []\n}\n", &edits);
        let twice = splice_ok(&once.text, &edits);

        assert_eq!(twice.text, once.text);
        assert!(twice.changed.is_empty(), "{:?}", twice.changed);
    }

    #[test]
    fn a_key_that_is_not_an_object_is_an_error_rather_than_a_clobber() {
        let error = splice(
            Path::new("settings.json"),
            "{\"profiles\": \"none\"}",
            &[Edit::Pair {
                key: key("profiles.defaults.colorScheme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::FromString,
            }],
        )
        .unwrap_err();

        assert!(
            matches!(&error, Error::KeyNotLocatable { key, .. } if key.to_string() == "profiles"),
            "{error}"
        );
        assert!(error.to_string().contains("profiles"), "{error}");
    }

    #[test]
    fn a_non_object_document_is_an_error() {
        for source in ["[]", "\"text\"", ""] {
            let error = splice(
                Path::new("settings.json"),
                source,
                &[Edit::Set {
                    key: key("theme"),
                    value: json!("dark"),
                }],
            )
            .unwrap_err();
            assert!(
                matches!(error, Error::KeyNotLocatable { .. }),
                "{source:?} gave {error}"
            );
        }
    }

    #[test]
    fn a_wrongly_typed_key_is_an_error() {
        let array = splice(
            Path::new("settings.json"),
            "{\"themes\": {}}",
            &[Edit::Element {
                key: key("themes"),
                name: "onecoat-dark".to_owned(),
                value: json!({ "name": "onecoat-dark" }),
            }],
        )
        .unwrap_err();
        assert!(
            matches!(&array, Error::KeyNotLocatable { key, expected, .. } if key.to_string() == "themes" && *expected == "a JSON array"),
            "{array}"
        );

        let number = splice(
            Path::new("settings.json"),
            "{\"theme\": 42}",
            &[Edit::Pair {
                key: key("theme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::FromString,
            }],
        )
        .unwrap_err();
        assert!(
            matches!(&number, Error::KeyNotLocatable { expected, .. } if *expected == "a JSON object or a string"),
            "{number}"
        );
    }

    #[test]
    fn a_broken_document_names_its_line_and_column() {
        let error = splice(
            Path::new("settings.json"),
            "{\n    \"a\": 1,\n    \"b\":\n}",
            &[Edit::Set {
                key: key("theme"),
                value: json!("dark"),
            }],
        )
        .unwrap_err();

        assert!(
            matches!(
                error,
                Error::JsoncUnparseable {
                    line: 4,
                    column: 1,
                    ..
                }
            ),
            "{error}"
        );
        assert!(error.to_string().contains("settings.json"), "{error}");
    }

    #[test]
    fn verify_accepts_valid_jsonc_and_reports_the_broken_line() {
        assert!(verify(Path::new("settings.json"), "{\n  // fine\n  \"a\": 1,\n}\n").is_ok());
        let error = verify(Path::new("settings.json"), "{\"a\": }").unwrap_err();
        assert!(
            matches!(error, Error::JsoncUnparseable { line: 1, .. }),
            "{error}"
        );
    }

    #[test]
    fn another_side_of_a_pair_keeps_the_side_it_does_not_own() {
        let source = "{\n  \"theme\": {\n    \"light\": \"light\"\n  }\n}\n";
        let spliced = splice_ok(
            source,
            &[Edit::Pair {
                key: key("theme"),
                side: Slot::Dark,
                name: "onecoat-dark".to_owned(),
                fill: Fill::BuiltIn,
            }],
        );

        assert_eq!(
            spliced.text,
            "{\n  \"theme\": {\n    \"light\": \"light\",\n    \"dark\": \"onecoat-dark\"\n  }\n}\n"
        );
    }
}
