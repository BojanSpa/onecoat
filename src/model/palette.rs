//! The base16 palette and the derived Windows Terminal ANSI set.

use std::ops::{Index, IndexMut};

use crate::model::color::HexColor;

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Base16Entry {
    B00,
    B01,
    B02,
    B03,
    B04,
    B05,
    B06,
    B07,
    B08,
    B09,
    B0A,
    B0B,
    B0C,
    B0D,
    B0E,
    B0F,
}

#[allow(missing_docs)]
impl Base16Entry {
    pub const ALL: [Self; 16] = [
        Self::B00,
        Self::B01,
        Self::B02,
        Self::B03,
        Self::B04,
        Self::B05,
        Self::B06,
        Self::B07,
        Self::B08,
        Self::B09,
        Self::B0A,
        Self::B0B,
        Self::B0C,
        Self::B0D,
        Self::B0E,
        Self::B0F,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::B00 => "base00",
            Self::B01 => "base01",
            Self::B02 => "base02",
            Self::B03 => "base03",
            Self::B04 => "base04",
            Self::B05 => "base05",
            Self::B06 => "base06",
            Self::B07 => "base07",
            Self::B08 => "base08",
            Self::B09 => "base09",
            Self::B0A => "base0A",
            Self::B0B => "base0B",
            Self::B0C => "base0C",
            Self::B0D => "base0D",
            Self::B0E => "base0E",
            Self::B0F => "base0F",
        }
    }

    pub fn from_name(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|entry| entry.name().eq_ignore_ascii_case(key))
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Palette([HexColor; 16]);

impl From<[HexColor; 16]> for Palette {
    fn from(colors: [HexColor; 16]) -> Self {
        Self(colors)
    }
}

impl Index<Base16Entry> for Palette {
    type Output = HexColor;

    fn index(&self, entry: Base16Entry) -> &Self::Output {
        &self.0[entry.index()]
    }
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnsiSlot {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightPurple,
    BrightCyan,
    BrightWhite,
}

#[allow(missing_docs)]
impl AnsiSlot {
    pub const ALL: [Self; 16] = [
        Self::Black,
        Self::Red,
        Self::Green,
        Self::Yellow,
        Self::Blue,
        Self::Purple,
        Self::Cyan,
        Self::White,
        Self::BrightBlack,
        Self::BrightRed,
        Self::BrightGreen,
        Self::BrightYellow,
        Self::BrightBlue,
        Self::BrightPurple,
        Self::BrightCyan,
        Self::BrightWhite,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::BrightBlack => "brightBlack",
            Self::BrightRed => "brightRed",
            Self::BrightGreen => "brightGreen",
            Self::BrightYellow => "brightYellow",
            Self::BrightBlue => "brightBlue",
            Self::BrightPurple => "brightPurple",
            Self::BrightCyan => "brightCyan",
            Self::BrightWhite => "brightWhite",
        }
    }

    pub fn from_name(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|slot| slot.name() == key)
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AnsiSet([HexColor; 16]);

impl From<[HexColor; 16]> for AnsiSet {
    fn from(colors: [HexColor; 16]) -> Self {
        Self(colors)
    }
}

impl Index<AnsiSlot> for AnsiSet {
    type Output = HexColor;

    fn index(&self, slot: AnsiSlot) -> &Self::Output {
        &self.0[slot.index()]
    }
}

impl IndexMut<AnsiSlot> for AnsiSet {
    fn index_mut(&mut self, slot: AnsiSlot) -> &mut Self::Output {
        &mut self.0[slot.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::{AnsiSlot, Base16Entry};

    #[test]
    fn base16_entries_resolve_case_insensitively() {
        assert_eq!(Base16Entry::from_name("base0a"), Some(Base16Entry::B0A));
        assert_eq!(Base16Entry::from_name("base0A"), Some(Base16Entry::B0A));
        assert_eq!(Base16Entry::from_name("base00"), Some(Base16Entry::B00));
        assert_eq!(Base16Entry::from_name("base10"), None);
        assert_eq!(Base16Entry::from_name("base"), None);
    }

    #[test]
    fn ansi_slots_use_the_windows_terminal_spelling() {
        assert_eq!(AnsiSlot::from_name("purple"), Some(AnsiSlot::Purple));
        assert_eq!(
            AnsiSlot::from_name("brightPurple"),
            Some(AnsiSlot::BrightPurple)
        );
        assert_eq!(AnsiSlot::from_name("magenta"), None);
        assert_eq!(AnsiSlot::from_name("Purple"), None);
    }
}
