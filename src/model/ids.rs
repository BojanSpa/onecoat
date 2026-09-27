use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use serde::Serialize;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
#[serde(transparent)]
pub struct ThemeId(String);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdProblem {
    Empty,
    BadChar(char),
}

impl ThemeId {
    pub fn parse(raw: &str) -> Result<Self, IdProblem> {
        let mut chars = raw.chars();
        let first = chars.next().ok_or(IdProblem::Empty)?;
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return Err(IdProblem::BadChar(first));
        }

        if let Some(ch) = chars.find(|ch| !is_id_continuation_char(*ch)) {
            return Err(IdProblem::BadChar(ch));
        }

        Ok(Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_id_continuation_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'
}

impl FromStr for ThemeId {
    type Err = IdProblem;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::parse(raw)
    }
}

impl fmt::Display for ThemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Borrow<str> for ThemeId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a theme id cannot be empty"),
            Self::BadChar(ch) => write!(
                f,
                "`{ch}` is not allowed; use lowercase ASCII letters, digits and hyphens"
            ),
        }
    }
}

impl std::error::Error for IdProblem {}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Dark,
    Light,
}

impl Slot {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Dark,
    Light,
}

impl From<Appearance> for Slot {
    fn from(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Dark => Slot::Dark,
            Appearance::Light => Slot::Light,
        }
    }
}

impl fmt::Display for Appearance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Slot::from(*self).name())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Wt,
    Herdr,
    Omp,
}

impl Target {
    pub const ALL: [Self; 3] = [Self::Wt, Self::Herdr, Self::Omp];

    pub fn from_name(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|target| target.name() == raw)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Wt => "wt",
            Self::Herdr => "herdr",
            Self::Omp => "omp",
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Bundled,
    User,
}

#[cfg(test)]
#[path = "../../tests/unit/model/ids.rs"]
mod tests;
