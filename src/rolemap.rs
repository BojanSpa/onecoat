//! Base16 role derivation shared by every renderer.
//!
//! [`derived_ansi`] is the single place where the palette-to-role mapping lives; a
//! renderer reads a role from it and never re-derives one, and `[ansi]` overrides are
//! applied on top of its result.

use crate::model::palette::{AnsiSet, Base16Entry, Palette};

/// Derives the Windows Terminal ANSI set from a palette.
pub fn derived_ansi(palette: &Palette) -> AnsiSet {
    AnsiSet::from([
        palette[Base16Entry::B00], // black
        palette[Base16Entry::B08], // red
        palette[Base16Entry::B0B], // green
        palette[Base16Entry::B0A], // yellow
        palette[Base16Entry::B0D], // blue
        palette[Base16Entry::B0E], // purple
        palette[Base16Entry::B0C], // cyan
        palette[Base16Entry::B06], // white
        palette[Base16Entry::B03], // brightBlack
        palette[Base16Entry::B08], // brightRed
        palette[Base16Entry::B0B], // brightGreen
        palette[Base16Entry::B0A], // brightYellow
        palette[Base16Entry::B0D], // brightBlue
        palette[Base16Entry::B0E], // brightPurple
        palette[Base16Entry::B0C], // brightCyan
        palette[Base16Entry::B07], // brightWhite
    ])
}

#[cfg(test)]
mod tests {
    use super::derived_ansi;
    use crate::model::color::HexColor;
    use crate::model::palette::{AnsiSlot, Base16Entry, Palette};

    /// A palette whose sixteen entries are all distinct.
    fn palette() -> Palette {
        Palette::from(
            Base16Entry::ALL
                .map(|entry| HexColor::parse(&format!("#{:02x}0000", entry.index())).unwrap()),
        )
    }

    #[test]
    fn every_ansi_role_follows_the_base16_table() {
        let palette = palette();
        let ansi = derived_ansi(&palette);
        for (slot, entry) in [
            (AnsiSlot::Black, Base16Entry::B00),
            (AnsiSlot::Red, Base16Entry::B08),
            (AnsiSlot::Green, Base16Entry::B0B),
            (AnsiSlot::Yellow, Base16Entry::B0A),
            (AnsiSlot::Blue, Base16Entry::B0D),
            (AnsiSlot::Purple, Base16Entry::B0E),
            (AnsiSlot::Cyan, Base16Entry::B0C),
            (AnsiSlot::White, Base16Entry::B06),
            (AnsiSlot::BrightBlack, Base16Entry::B03),
            (AnsiSlot::BrightRed, Base16Entry::B08),
            (AnsiSlot::BrightGreen, Base16Entry::B0B),
            (AnsiSlot::BrightYellow, Base16Entry::B0A),
            (AnsiSlot::BrightBlue, Base16Entry::B0D),
            (AnsiSlot::BrightPurple, Base16Entry::B0E),
            (AnsiSlot::BrightCyan, Base16Entry::B0C),
            (AnsiSlot::BrightWhite, Base16Entry::B07),
        ] {
            assert_eq!(ansi[slot], palette[entry], "role {}", slot.name());
        }
    }
}
