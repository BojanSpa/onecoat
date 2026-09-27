use serde::Serialize;
use serde_json::Value;

use crate::Error;
use crate::jsonc::{Edit, Fill, Key};
use crate::model::color::HexColor;
use crate::model::ids::Slot;
use crate::model::palette::{AnsiSlot, Base16Entry};
use crate::model::theme::{Theme, Validated};

const SCHEMES_KEY: &str = "schemes";
const THEME_KEY: &str = "theme";
const THEMES_KEY: &str = "themes";
const COLOR_SCHEME_KEY: &str = "profiles.defaults.colorScheme";

pub const PROFILES_KEY: &str = "profiles.list";
pub const COLOR_SCHEME_FIELD: &str = "colorScheme";

#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum ProfileScheme {
    Report,
    All,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct WtScheme {
    name: String,
    background: HexColor,
    foreground: HexColor,
    cursorColor: HexColor,
    selectionBackground: HexColor,
    black: HexColor,
    red: HexColor,
    green: HexColor,
    yellow: HexColor,
    blue: HexColor,
    purple: HexColor,
    cyan: HexColor,
    white: HexColor,
    brightBlack: HexColor,
    brightRed: HexColor,
    brightGreen: HexColor,
    brightYellow: HexColor,
    brightBlue: HexColor,
    brightPurple: HexColor,
    brightCyan: HexColor,
    brightWhite: HexColor,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct WtWindowTheme {
    name: String,
    window: WtWindow,
    tabRow: WtTabRow,
    tab: WtTab,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct WtWindow {
    applicationTheme: String,
    frame: HexColor,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct WtTabRow {
    background: HexColor,
    unfocusedBackground: HexColor,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct WtTab {
    background: HexColor,
    unfocusedBackground: HexColor,
}

pub fn scheme(theme: &Theme<Validated>) -> Result<Value, Error> {
    let data = &theme.data;
    let palette = &data.palette;
    let ansi = &data.ansi;

    to_value(&WtScheme {
        name: name(theme),
        background: background(theme),
        foreground: data.wt.foreground.unwrap_or(palette[Base16Entry::B05]),
        cursorColor: data.wt.cursor_color.unwrap_or(palette[Base16Entry::B0D]),
        selectionBackground: data
            .wt
            .selection_background
            .unwrap_or(palette[Base16Entry::B02]),
        black: ansi[AnsiSlot::Black],
        red: ansi[AnsiSlot::Red],
        green: ansi[AnsiSlot::Green],
        yellow: ansi[AnsiSlot::Yellow],
        blue: ansi[AnsiSlot::Blue],
        purple: ansi[AnsiSlot::Purple],
        cyan: ansi[AnsiSlot::Cyan],
        white: ansi[AnsiSlot::White],
        brightBlack: ansi[AnsiSlot::BrightBlack],
        brightRed: ansi[AnsiSlot::BrightRed],
        brightGreen: ansi[AnsiSlot::BrightGreen],
        brightYellow: ansi[AnsiSlot::BrightYellow],
        brightBlue: ansi[AnsiSlot::BrightBlue],
        brightPurple: ansi[AnsiSlot::BrightPurple],
        brightCyan: ansi[AnsiSlot::BrightCyan],
        brightWhite: ansi[AnsiSlot::BrightWhite],
    })
}

pub fn window_theme(theme: &Theme<Validated>) -> Result<Value, Error> {
    let palette = &theme.data.palette;

    to_value(&WtWindowTheme {
        name: name(theme),
        window: WtWindow {
            applicationTheme: Slot::from(theme.appearance).name().to_owned(),
            frame: background(theme),
        },
        tabRow: WtTabRow {
            background: palette[Base16Entry::B01],
            unfocusedBackground: background(theme),
        },
        tab: WtTab {
            background: palette[Base16Entry::B01],
            unfocusedBackground: background(theme),
        },
    })
}

pub fn fragment_edits(theme: &Theme<Validated>) -> Result<Vec<Edit>, Error> {
    Ok(vec![Edit::Element {
        key: Key::parse(SCHEMES_KEY),
        name: name(theme),
        value: scheme(theme)?,
    }])
}

pub fn settings_edits(
    theme: &Theme<Validated>,
    profile_scheme: ProfileScheme,
) -> Result<Vec<Edit>, Error> {
    let side = Slot::from(theme.appearance);

    let mut edits = vec![
        Edit::Pair {
            key: Key::parse(THEME_KEY),
            side,
            name: name(theme),
            fill: Fill::BuiltIn,
        },
        Edit::Element {
            key: Key::parse(THEMES_KEY),
            name: name(theme),
            value: window_theme(theme)?,
        },
        Edit::Pair {
            key: Key::parse(COLOR_SCHEME_KEY),
            side,
            name: name(theme),
            fill: Fill::FromString,
        },
    ];

    if profile_scheme == ProfileScheme::All {
        edits.push(Edit::Repoint {
            key: Key::parse(PROFILES_KEY),
            field: COLOR_SCHEME_FIELD.to_owned(),
            side,
            name: name(theme),
            fill: Fill::FromString,
        });
    }

    Ok(edits)
}

pub fn name(theme: &Theme<Validated>) -> String {
    format!("onecoat-{}", Slot::from(theme.appearance).name())
}

fn background(theme: &Theme<Validated>) -> HexColor {
    theme
        .data
        .wt
        .background
        .unwrap_or(theme.data.palette[Base16Entry::B00])
}

fn to_value<T: Serialize>(value: &T) -> Result<Value, Error> {
    serde_json::to_value(value).map_err(|source| Error::JsonEncodeFailed { source })
}

#[cfg(test)]
#[path = "../../tests/unit/render/wt.rs"]
mod tests;
