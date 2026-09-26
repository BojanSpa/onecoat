//! The Windows Terminal colour scheme.
//!
//! This module is the only place where Windows Terminal's key spellings exist: the
//! magenta role is `purple` and `brightPurple`, because that is what the schema says.

use serde::Serialize;

use crate::model::color::HexColor;
use crate::model::ids::Slot;
use crate::model::palette::{AnsiSlot, Base16Entry};
use crate::model::theme::{Theme, Validated};

/// One colour scheme; the field order is the key order in the fragment file.
///
/// The field names are Windows Terminal's own scheme keys, spelled exactly as its
/// schema defines them, so the struct is the only place a key can be misspelled.
#[allow(non_snake_case, missing_docs)]
#[derive(Serialize)]
pub struct WtScheme {
    pub name: String,
    pub background: HexColor,
    pub foreground: HexColor,
    pub cursorColor: HexColor,
    pub selectionBackground: HexColor,
    pub black: HexColor,
    pub red: HexColor,
    pub green: HexColor,
    pub yellow: HexColor,
    pub blue: HexColor,
    pub purple: HexColor,
    pub cyan: HexColor,
    pub white: HexColor,
    pub brightBlack: HexColor,
    pub brightRed: HexColor,
    pub brightGreen: HexColor,
    pub brightYellow: HexColor,
    pub brightBlue: HexColor,
    pub brightPurple: HexColor,
    pub brightCyan: HexColor,
    pub brightWhite: HexColor,
}

/// The Windows Terminal fragment onecoat owns.
#[allow(missing_docs)]
#[derive(Serialize)]
pub struct WtFragment {
    pub schemes: Vec<WtScheme>,
}

impl WtFragment {
    /// Renders the theme's declared slot as one scheme named `onecoat-<slot>`.
    ///
    /// The name is stable across theme switches, because it depends on the slot and not
    /// on the theme.
    pub fn for_theme(theme: &Theme<Validated>) -> Self {
        let data = &theme.data;
        let palette = &data.palette;
        let ansi = &data.ansi;
        Self {
            schemes: vec![WtScheme {
                name: format!("onecoat-{}", Slot::from(theme.appearance).name()),
                background: data.wt.background.unwrap_or(palette[Base16Entry::B00]),
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
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::WtFragment;
    use crate::model::color::HexColor;
    use crate::model::ids::{Origin, Slot};
    use crate::model::palette::{AnsiSlot, Base16Entry};
    use crate::model::theme::{Theme, Validated};

    /// Parses and validates a theme source.
    fn theme(name: &str, source: &str) -> Theme<Validated> {
        Theme::parse(
            PathBuf::from(format!("themes/{name}.toml")),
            source,
            Origin::Bundled,
        )
        .unwrap()
        .validate()
        .unwrap()
    }

    #[test]
    fn every_scheme_key_carries_its_base16_role() {
        let theme = theme("nord", crate::themes::BUNDLED[0].1);
        let mut schemes = WtFragment::for_theme(&theme).schemes;
        assert_eq!(schemes.len(), 1);
        let scheme = schemes.pop().unwrap();
        let palette = &theme.data.palette;
        let ansi = &theme.data.ansi;

        assert_eq!(scheme.name, "onecoat-dark");
        assert_eq!(Slot::from(theme.appearance).name(), "dark");
        assert_eq!(scheme.background, palette[Base16Entry::B00]);
        assert_eq!(scheme.foreground, palette[Base16Entry::B05]);
        assert_eq!(scheme.cursorColor, palette[Base16Entry::B0D]);
        assert_eq!(scheme.selectionBackground, palette[Base16Entry::B02]);
        for (actual, slot) in [
            (scheme.black, AnsiSlot::Black),
            (scheme.red, AnsiSlot::Red),
            (scheme.green, AnsiSlot::Green),
            (scheme.yellow, AnsiSlot::Yellow),
            (scheme.blue, AnsiSlot::Blue),
            (scheme.purple, AnsiSlot::Purple),
            (scheme.cyan, AnsiSlot::Cyan),
            (scheme.white, AnsiSlot::White),
            (scheme.brightBlack, AnsiSlot::BrightBlack),
            (scheme.brightRed, AnsiSlot::BrightRed),
            (scheme.brightGreen, AnsiSlot::BrightGreen),
            (scheme.brightYellow, AnsiSlot::BrightYellow),
            (scheme.brightBlue, AnsiSlot::BrightBlue),
            (scheme.brightPurple, AnsiSlot::BrightPurple),
            (scheme.brightCyan, AnsiSlot::BrightCyan),
            (scheme.brightWhite, AnsiSlot::BrightWhite),
        ] {
            assert_eq!(actual, ansi[slot], "scheme key {}", slot.name());
        }
        assert_eq!(scheme.white, palette[Base16Entry::B06]);
        assert_eq!(scheme.brightWhite, palette[Base16Entry::B07]);
        assert_eq!(scheme.purple, palette[Base16Entry::B0E]);
    }

    #[test]
    fn overrides_replace_only_the_roles_they_name() {
        let theme = theme(
            "overrides",
            include_str!("../../tests/fixtures/themes/overrides.toml"),
        );
        let mut schemes = WtFragment::for_theme(&theme).schemes;
        let scheme = schemes.pop().unwrap();

        assert_eq!(scheme.red, HexColor::parse("#ff0000").unwrap());
        assert_eq!(
            scheme.green,
            theme.data.palette[Base16Entry::B0B],
            "an ANSI slot the file did not override keeps its derived colour"
        );
        assert_eq!(scheme.cursorColor, HexColor::parse("#d8dee9").unwrap());
        assert_eq!(
            scheme.background,
            theme.data.palette[Base16Entry::B00],
            "the background stays derived when `[targets.wt]` does not override it"
        );
    }
}
