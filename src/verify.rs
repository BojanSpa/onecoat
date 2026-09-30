use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::{Map, Value};

use crate::Error;
use crate::jsonc::{self, Edit, Key};
use crate::model::ids::{Slot, Target};
use crate::plan::{Absent, Content, PlannedWrite};
use crate::render::{herdr, omp};

const COLUMNS: usize = 5;
const HEADER: [&str; COLUMNS] = ["target", "slot", "key", "expected", "found"];
const NO_DRIFT: &str = "no drift";
const NO_VALUE: &str = "-";
const OR: &str = " or ";

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct Finding {
    pub target: Target,
    pub slot: Slot,
    pub key: String,
    pub expected: Option<String>,
    pub found: Option<String>,
}

pub struct Alternatives(BTreeMap<String, Vec<String>>);

pub fn shared_alternatives<'a>(writes: impl Iterator<Item = &'a PlannedWrite>) -> Alternatives {
    let mut shared: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for write in writes {
        let Content::Toml(edits) = &write.content else {
            continue;
        };

        for edit in edits.iter().filter(|edit| herdr::is_shared(edit)) {
            let values = shared.entry(edit.path.join(".")).or_default();
            let rendered = text(&edit.value);
            if !values.contains(&rendered) {
                values.push(rendered);
            }
        }
    }

    Alternatives(shared)
}

pub fn drift(
    slot: Slot,
    write: &PlannedWrite,
    source: Option<&str>,
    alternatives: &Alternatives,
) -> Result<Vec<Finding>, Error> {
    if source.is_none() && write.absent == Absent::Fail {
        return Err(Error::MissingFile {
            path: write.path.clone(),
        });
    }

    let mut drift = Drift {
        target: write.target,
        slot,
        findings: Vec::new(),
    };

    match &write.content {
        Content::Jsonc(edits) => {
            for edit in edits {
                let Some((key, expected)) = jsonc_expectation(edit) else {
                    continue;
                };

                let found = source
                    .map(|source| jsonc::value(&write.path, source, &key))
                    .transpose()?
                    .flatten();

                drift.compare(&key.to_string(), Some(&expected), found.as_ref());
            }
        }
        Content::Generated(generated) => {
            let expected = expected_document(generated)?;
            let found = source
                .map(|source| jsonc::value(&write.path, source, &Key::root()))
                .transpose()?
                .flatten();

            drift.compare("", Some(&expected), found.as_ref());
        }
        Content::Toml(edits) => {
            let parsed = source
                .map(|source| herdr::parse(&write.path, source))
                .transpose()?;

            for edit in edits {
                let key = edit.path.join(".");

                let found = parsed
                    .as_ref()
                    .and_then(|parsed| herdr::value(parsed, &edit.path));

                if herdr::is_shared(edit)
                    && let Some(values) = alternatives.0.get(&key)
                {
                    drift.shared(&key, values, found.as_ref());
                    continue;
                }

                drift.compare(
                    &key,
                    Some(&Value::String(text(&edit.value))),
                    found.as_ref(),
                );
            }
        }
        Content::Yaml(edits) => {
            for edit in edits {
                let key = format!("{}.{}", omp::THEME_KEY, edit.side.name());

                let found = source
                    .map(|source| omp::config_value(&write.path, source, edit.side))
                    .transpose()?
                    .flatten()
                    .map(Value::String);

                drift.compare(
                    &key,
                    Some(&Value::String(edit.name.clone())),
                    found.as_ref(),
                );
            }
        }
    }

    Ok(drift.findings)
}

pub fn collapse(findings: Vec<Finding>) -> Vec<Finding> {
    let mut kept: Vec<Finding> = Vec::new();
    for finding in findings {
        let repeated = kept.iter().any(|other| {
            other.target == finding.target
                && other.key == finding.key
                && other.expected == finding.expected
                && other.found == finding.found
        });

        if !repeated {
            kept.push(finding);
        }
    }

    kept
}

pub fn table(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return NO_DRIFT.to_owned();
    }

    let rows: Vec<[String; COLUMNS]> = std::iter::once(HEADER.map(str::to_owned))
        .chain(findings.iter().map(row))
        .collect();

    let widths: Vec<usize> = (0..COLUMNS)
        .map(|index| {
            rows.iter()
                .map(|row| row[index].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();

    rows.iter()
        .map(|row| pad(row, &widths))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn json(findings: &[Finding]) -> Result<String, Error> {
    serde_json::to_string_pretty(findings).map_err(|source| Error::JsonEncodeFailed { source })
}

struct Drift {
    target: Target,
    slot: Slot,
    findings: Vec<Finding>,
}

impl Drift {
    fn compare(&mut self, key: &str, expected: Option<&Value>, found: Option<&Value>) {
        if let (Some(Value::Object(expected)), Some(Value::Object(found))) = (expected, found) {
            for name in union(expected, found) {
                let child = child_key(key, &name);
                let expected = expected.get(&name);
                let found = found.get(&name);

                self.compare(&child, expected, found);
            }

            return;
        }

        if let (Some(Value::Object(object)), None) = (expected, found) {
            for name in names(object) {
                let child = child_key(key, &name);
                let value = object.get(&name);

                self.compare(&child, value, None);
            }

            return;
        }

        if let (None, Some(Value::Object(object))) = (expected, found) {
            for name in names(object) {
                let child = child_key(key, &name);
                let value = object.get(&name);

                self.compare(&child, None, value);
            }

            return;
        }

        if expected != found {
            self.push(key, expected.map(render), found.map(render));
        }
    }

    fn shared(&mut self, key: &str, values: &[String], found: Option<&Value>) {
        if let Some(found) = found
            && values.contains(&render(found))
        {
            return;
        }

        self.push(key, Some(values.join(OR)), found.map(render));
    }

    fn push(&mut self, key: &str, expected: Option<String>, found: Option<String>) {
        self.findings.push(Finding {
            target: self.target,
            slot: self.slot,
            key: key.to_owned(),
            expected,
            found,
        });
    }
}

fn jsonc_expectation(edit: &Edit) -> Option<(Key, Value)> {
    match edit {
        Edit::Set { key, value } => Some((key.clone(), value.clone())),
        Edit::Element { key, name, value } => Some((key.join(name), value.clone())),
        Edit::Pair {
            key, side, name, ..
        } => Some((key.join(side.name()), Value::String(name.clone()))),
        Edit::Repoint { .. } => None,
    }
}

fn expected_document(text: &str) -> Result<Value, Error> {
    serde_json::from_str(text).map_err(|source| Error::JsonEncodeFailed { source })
}

fn text(value: &herdr::Value) -> String {
    match value {
        herdr::Value::Color(color) => color.to_string(),
        herdr::Value::Bool(flag) => flag.to_string(),
        herdr::Value::Text(text) => text.clone(),
    }
}

fn union(expected: &Map<String, Value>, found: &Map<String, Value>) -> Vec<String> {
    let mut names: BTreeSet<String> = expected.keys().cloned().collect();
    names.extend(found.keys().cloned());

    names.into_iter().collect()
}

fn names(object: &Map<String, Value>) -> Vec<String> {
    let mut names: Vec<String> = object.keys().cloned().collect();
    names.sort();

    names
}

fn child_key(key: &str, name: &str) -> String {
    if key.is_empty() {
        return name.to_owned();
    }

    format!("{key}.{name}")
}

fn render(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn row(finding: &Finding) -> [String; COLUMNS] {
    [
        finding.target.to_string(),
        finding.slot.name().to_owned(),
        finding.key.clone(),
        finding
            .expected
            .clone()
            .unwrap_or_else(|| NO_VALUE.to_owned()),
        finding.found.clone().unwrap_or_else(|| NO_VALUE.to_owned()),
    ]
}

fn pad(row: &[String; COLUMNS], widths: &[usize]) -> String {
    row.iter()
        .zip(widths)
        .map(|(cell, width)| format!("{cell:<width$}"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end()
        .to_owned()
}

#[cfg(test)]
#[path = "../tests/unit/verify.rs"]
mod tests;
