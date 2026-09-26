//! Base16 role derivation shared by every renderer.
//!
//! [`derived_ansi`] is the single place renderers get the Windows Terminal role set
//! from; a renderer reads a role and never re-derives one, and `[ansi]` overrides are
//! applied on top of its result.

use crate::model::palette::{AnsiSet, AnsiSlot, Base16Entry, Palette};

/// Derives the Windows Terminal ANSI set from a palette.
pub fn derived_ansi(palette: &Palette) -> AnsiSet {
    AnsiSet::from(AnsiSlot::ALL.map(|slot| palette[source_entry(slot)]))
}

/// The base16 entry each Windows Terminal role is derived from.
fn source_entry(slot: AnsiSlot) -> Base16Entry {
    match slot {
        AnsiSlot::Black => Base16Entry::B00,
        AnsiSlot::Red => Base16Entry::B08,
        AnsiSlot::Green => Base16Entry::B0B,
        AnsiSlot::Yellow => Base16Entry::B0A,
        AnsiSlot::Blue => Base16Entry::B0D,
        AnsiSlot::Purple => Base16Entry::B0E,
        AnsiSlot::Cyan => Base16Entry::B0C,
        AnsiSlot::White => Base16Entry::B06,
        AnsiSlot::BrightBlack => Base16Entry::B03,
        AnsiSlot::BrightRed => Base16Entry::B08,
        AnsiSlot::BrightGreen => Base16Entry::B0B,
        AnsiSlot::BrightYellow => Base16Entry::B0A,
        AnsiSlot::BrightBlue => Base16Entry::B0D,
        AnsiSlot::BrightPurple => Base16Entry::B0E,
        AnsiSlot::BrightCyan => Base16Entry::B0C,
        AnsiSlot::BrightWhite => Base16Entry::B07,
    }
}

#[cfg(test)]
mod tests {
    use super::derived_ansi;
    use crate::model::color::HexColor;
    use crate::model::palette::{AnsiSlot, Base16Entry, Palette};

    fn distinct_palette() -> Palette {
        Palette::from(
            Base16Entry::ALL
                .map(|entry| HexColor::parse(&format!("#{:02x}0000", entry.index())).unwrap()),
        )
    }

    #[test]
    fn every_ansi_role_follows_the_base16_table() {
        let palette = distinct_palette();
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
