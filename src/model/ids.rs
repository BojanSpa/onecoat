use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use serde::Serialize;

#[allow(missing_docs)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
#[serde(transparent)]
pub struct ThemeId(String);

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdProblem {
    Empty,
    BadChar(char),
}

#[allow(missing_docs)]
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

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Dark,
    Light,
}

impl Slot {
    #[allow(missing_docs)]
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

#[allow(missing_docs)]
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

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Wt,
    Herdr,
    Omp,
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Bundled,
    User,
}

#[cfg(test)]
mod tests {
    use super::{IdProblem, Slot, ThemeId};

    #[test]
    fn theme_id_accepts_lowercase_ids() {
        assert_eq!(ThemeId::parse("nord").unwrap().as_str(), "nord");
        assert_eq!(ThemeId::parse("nord-2").unwrap().as_str(), "nord-2");
        assert_eq!(ThemeId::parse("2-tone").unwrap().as_str(), "2-tone");
        assert_eq!(ThemeId::parse("nord-").unwrap().as_str(), "nord-");
    }

    #[test]
    fn theme_id_rejects_an_empty_id() {
        assert_eq!(ThemeId::parse(""), Err(IdProblem::Empty));
    }

    #[test]
    fn theme_id_rejects_characters_outside_lowercase_ascii() {
        assert_eq!(ThemeId::parse("NORD"), Err(IdProblem::BadChar('N')));
        assert_eq!(ThemeId::parse("nord-å"), Err(IdProblem::BadChar('å')));
    }

    #[test]
    fn theme_id_rejects_a_leading_hyphen() {
        assert_eq!(ThemeId::parse("-nord"), Err(IdProblem::BadChar('-')));
    }

    #[test]
    fn appearance_names_its_slot() {
        assert_eq!(Slot::from(super::Appearance::Dark).name(), "dark");
        assert_eq!(Slot::from(super::Appearance::Light).name(), "light");
    }
}
