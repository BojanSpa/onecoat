use std::{io, path::PathBuf};

use crate::jsonc::Key;
use crate::model::{Appearance, Base16Entry, HexColor, ThemeId};
use crate::state::Problem;

pub(crate) fn accepted_keys(list: &[&str]) -> String {
    list.join(", ")
}

fn missing_base16_names(missing: &[Base16Entry]) -> String {
    missing
        .iter()
        .copied()
        .map(Base16Entry::name)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("theme `{id}` not found; run `onecoat list` to see the available themes")]
    ThemeNotFound { id: ThemeId },

    #[error("environment variable `{var}` is not set; onecoat needs it to locate the target paths")]
    EnvMissing { var: &'static str },

    #[error(
        "cannot read `{path}`: {source}; check that the path is readable and not locked by another program"
    )]
    FileUnreadable { path: PathBuf, source: io::Error },

    #[error(
        "cannot write `{path}`: {source}; check that the path is writable and not open in another program"
    )]
    FileWriteFailed { path: PathBuf, source: io::Error },

    #[error(
        "`{path}` does not exist; onecoat only edits this file and never invents one, so run the program that owns it once and retry"
    )]
    MissingFile { path: PathBuf },

    #[error(
        "`{path}` is not valid JSONC: {message} at line {line}, column {column}; fix the syntax at that position"
    )]
    JsoncUnparseable {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },

    #[error(
        "`{path}`: `{key}` is not {expected}; onecoat never overwrites a value it does not own, so fix that key by hand"
    )]
    KeyNotLocatable {
        path: PathBuf,
        key: Key,
        expected: &'static str,
    },

    #[error(
        "`{path}` does not parse after onecoat wrote it ({source}); the previous content was restored from `{backup}`"
    )]
    ApplyNotVerified {
        path: PathBuf,
        backup: PathBuf,
        source: Box<Error>,
    },

    #[error(
        "`{path}` does not parse after onecoat wrote it and `{backup}` is missing; nothing was restored, so fix the file and its backup by hand"
    )]
    BackupMissing { path: PathBuf, backup: PathBuf },

    #[error("cannot create directory `{path}`: {source}; create it by hand or fix its permissions")]
    DirCreateFailed { path: PathBuf, source: io::Error },

    #[error(
        "`{path}` is not valid theme TOML: {source}; fix the syntax at the reported line and column"
    )]
    ThemeUnparseable {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("`{path}`: `{key}` is missing; add `{key}` to the theme file")]
    MissingField { path: PathBuf, key: &'static str },

    #[error("`{path}`: `{key}` must be {expected}; found {found}. Fix the value in the theme file")]
    FieldType {
        path: PathBuf,
        key: &'static str,
        expected: &'static str,
        found: &'static str,
    },

    #[error("`{path}`: appearance = \"{value}\" is not an appearance; use \"dark\" or \"light\"")]
    UnknownAppearance { path: PathBuf, value: String },

    #[error(
        "`{path}`: the theme id is empty; use lowercase ASCII letters, digits and hyphens, starting with a letter or digit"
    )]
    ThemeIdEmpty { path: PathBuf },

    #[error(
        "`{path}`: theme id `{id}` contains `{ch}`; use lowercase ASCII letters, digits and hyphens, starting with a letter or digit"
    )]
    ThemeIdInvalidChar { path: PathBuf, id: String, ch: char },

    #[error("`{path}`: theme id `{id}` collides with the omp built-in theme `{id}`; rename the id")]
    ReservedId { path: PathBuf, id: ThemeId },

    #[error("`{path}`: `{key}` = {value} is not a colour; write it as \"#rrggbb\"")]
    MalformedColor {
        path: PathBuf,
        key: String,
        value: String,
    },

    #[error(
        "`{path}`: `{key}` = {value} is not a colour or a token value; use \"#rrggbb\", an integer from 0 to 255, or a string"
    )]
    OverrideValueInvalid {
        path: PathBuf,
        key: String,
        value: String,
    },

    #[error("[palette] in `{path}` is missing {}; add every base16 entry from base00 to base0F", missing_base16_names(.missing))]
    PaletteIncomplete {
        path: PathBuf,
        missing: Vec<Base16Entry>,
    },

    #[error(
        "`{path}`: appearance = \"{declared}\" disagrees with base00 = \"{background}\" (relative luminance {luminance:.3}); set appearance = \"{expected}\""
    )]
    AppearanceMismatch {
        path: PathBuf,
        background: HexColor,
        declared: Appearance,
        luminance: f64,
        expected: Appearance,
    },

    #[error("`{path}`: unknown key `{key}` in {section}; accepted keys are {accepted}")]
    UnknownKey {
        path: PathBuf,
        section: &'static str,
        key: String,
        accepted: String,
    },

    #[error(
        "`{path}`: theme id `{id}` is already declared by `{other}`; give one of the two files a different id"
    )]
    DuplicateThemeId {
        path: PathBuf,
        other: PathBuf,
        id: ThemeId,
    },

    #[error(
        "`{path}` is not valid state: {problem}; delete the file to reset the state, which is disposable"
    )]
    StateInvalid { path: PathBuf, problem: Problem },

    #[error("cannot encode JSON output: {source}; this is a bug in onecoat, please report it")]
    JsonEncodeFailed { source: serde_json::Error },
}
