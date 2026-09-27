use std::fmt;

use serde::Serialize;
use serde_json::Value;

use crate::Error;
use crate::model::ids::{IdProblem, Slot, Target, ThemeId};

const ROOT_KEYS: [&str; 2] = ["slots", "targets"];
const SLOT_KEYS: [&str; 2] = ["dark", "light"];
const SLOTS_KEY: &str = "slots";
const TARGETS_KEY: &str = "targets";
const DARK_KEY: &str = "slots.dark";
const LIGHT_KEY: &str = "slots.light";
const AN_OBJECT: &str = "an object";
const AN_ARRAY: &str = "an array of target names";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct State {
    dark: Option<ThemeId>,
    light: Option<ThemeId>,
    targets: Vec<Target>,
}

#[derive(Debug)]
pub enum Problem {
    Document(serde_json::Error),
    Missing {
        key: &'static str,
    },
    Type {
        key: &'static str,
        expected: &'static str,
    },
    Unknown {
        key: String,
    },
    Id {
        key: &'static str,
        problem: IdProblem,
    },
    Target {
        name: String,
    },
}

#[derive(Serialize)]
struct Document<'a> {
    slots: Slots<'a>,
    targets: &'a [Target],
}

#[derive(Serialize)]
struct Slots<'a> {
    dark: Option<&'a ThemeId>,
    light: Option<&'a ThemeId>,
}

impl State {
    pub fn none() -> Self {
        Self {
            dark: None,
            light: None,
            targets: Vec::new(),
        }
    }

    pub fn parse(text: &str) -> Result<Self, Problem> {
        let root: Value = serde_json::from_str(text).map_err(Problem::Document)?;

        let Value::Object(root) = root else {
            return Err(Problem::Type {
                key: "the document",
                expected: AN_OBJECT,
            });
        };

        if let Some(key) = root.keys().find(|key| !ROOT_KEYS.contains(&key.as_str())) {
            return Err(Problem::Unknown { key: key.clone() });
        }

        let Some(Value::Object(slots)) = root.get(SLOTS_KEY) else {
            return Err(missing_or_type(&root, SLOTS_KEY, AN_OBJECT));
        };

        if let Some(key) = slots.keys().find(|key| !SLOT_KEYS.contains(&key.as_str())) {
            return Err(Problem::Unknown {
                key: format!("{SLOTS_KEY}.{key}"),
            });
        }

        let Some(Value::Array(items)) = root.get(TARGETS_KEY) else {
            return Err(missing_or_type(&root, TARGETS_KEY, AN_ARRAY));
        };

        Ok(Self {
            dark: slot_assignment(slots, "dark", DARK_KEY)?,
            light: slot_assignment(slots, "light", LIGHT_KEY)?,
            targets: items
                .iter()
                .map(target_name)
                .collect::<Result<Vec<Target>, Problem>>()?,
        })
    }

    pub fn render(&self) -> Result<String, Error> {
        let document = Document {
            slots: Slots {
                dark: self.dark.as_ref(),
                light: self.light.as_ref(),
            },
            targets: &self.targets,
        };

        let mut text = serde_json::to_string_pretty(&document)
            .map_err(|source| Error::JsonEncodeFailed { source })?;

        text.push('\n');

        Ok(text)
    }

    pub fn assigned(&self, slot: Slot) -> Option<&ThemeId> {
        match slot {
            Slot::Dark => self.dark.as_ref(),
            Slot::Light => self.light.as_ref(),
        }
    }

    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    pub fn assign(&mut self, slot: Slot, id: ThemeId) {
        match slot {
            Slot::Dark => self.dark = Some(id),
            Slot::Light => self.light = Some(id),
        }
    }

    pub fn record(&mut self, targets: impl IntoIterator<Item = Target>) {
        self.targets = targets.into_iter().collect();
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(source) => write!(f, "it is not valid JSON ({source})"),
            Self::Missing { key } => write!(f, "the key `{key}` is missing"),
            Self::Type { key, expected } => write!(f, "`{key}` must be {expected}"),
            Self::Unknown { key } => write!(f, "`{key}` is not a state key"),
            Self::Id { key, problem } => write!(f, "`{key}` {problem}"),
            Self::Target { name } => write!(f, "`{name}` is not a target; use wt, herdr or omp"),
        }
    }
}

impl std::error::Error for Problem {}

fn missing_or_type(
    root: &serde_json::Map<String, Value>,
    key: &'static str,
    expected: &'static str,
) -> Problem {
    if root.contains_key(key) {
        Problem::Type { key, expected }
    } else {
        Problem::Missing { key }
    }
}

fn slot_assignment(
    slots: &serde_json::Map<String, Value>,
    field: &str,
    key: &'static str,
) -> Result<Option<ThemeId>, Problem> {
    match slots.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(raw)) => ThemeId::parse(raw)
            .map(Some)
            .map_err(|problem| Problem::Id { key, problem }),
        Some(_) => Err(Problem::Type {
            key,
            expected: "a theme id or null",
        }),
    }
}

fn target_name(value: &Value) -> Result<Target, Problem> {
    let name = value.as_str().ok_or(Problem::Type {
        key: TARGETS_KEY,
        expected: AN_ARRAY,
    })?;

    Target::from_name(name).ok_or_else(|| Problem::Target {
        name: name.to_owned(),
    })
}

#[cfg(test)]
#[path = "../tests/unit/state.rs"]
mod tests;
