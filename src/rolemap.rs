use crate::model::palette::{AnsiSet, AnsiSlot, Base16Entry, Palette};

pub fn derived_ansi(palette: &Palette) -> AnsiSet {
    AnsiSet::from(AnsiSlot::ALL.map(|slot| palette[source_entry(slot)]))
}

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
#[path = "../tests/unit/rolemap.rs"]
mod tests;
