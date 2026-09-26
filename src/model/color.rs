//! The colour type: parsed at the boundary, never re-parsed by a renderer.

use std::fmt;

use serde::{Serialize, Serializer};

/// An opaque sRGB colour, written as `#rrggbb`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HexColor {
    r: u8,
    g: u8,
    b: u8,
}

impl HexColor {
    /// Parses `#` followed by exactly six hex digits, either case.
    ///
    /// `#rgb` and `#rrggbbaa` are rejected, because Windows Terminal rejects them.
    pub fn parse(raw: &str) -> Option<Self> {
        let digits = raw.strip_prefix('#')?;
        if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let channel =
            |range: std::ops::Range<usize>| u8::from_str_radix(digits.get(range)?, 16).ok();
        Some(Self {
            r: channel(0..2)?,
            g: channel(2..4)?,
            b: channel(4..6)?,
        })
    }

    /// Builds a colour from its channels.
    ///
    /// Parse a file instead wherever possible; `validate` uses this only to fill
    /// palette slots that are overwritten in the same pass.
    pub(crate) const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// WCAG 2.x relative luminance of the linearised sRGB channels, in `0.0..=1.0`.
    pub fn luminance(self) -> f64 {
        fn linearise(channel: u8) -> f64 {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linearise(self.r) + 0.7152 * linearise(self.g) + 0.0722 * linearise(self.b)
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl Serialize for HexColor {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::HexColor;

    #[test]
    fn parse_accepts_six_hex_digits_in_either_case() {
        assert_eq!(HexColor::parse("#0B1018"), HexColor::parse("#0b1018"));
        assert!(HexColor::parse("#0B1018").is_some());
        assert!(HexColor::parse("#0b1018").is_some());
    }

    #[test]
    fn parse_rejects_shapes_windows_terminal_rejects() {
        for raw in ["#abc", "#0b1018ff", "0b1018", "#gggggg", "#", ""] {
            assert_eq!(HexColor::parse(raw), None, "{raw:?} must not parse");
        }
    }

    #[test]
    fn display_writes_a_lowercase_hash_colour() {
        assert_eq!(HexColor::parse("#0B1018").unwrap().to_string(), "#0b1018");
    }

    #[test]
    fn luminance_spans_the_full_range() {
        let black = HexColor::parse("#000000").unwrap().luminance();
        let white = HexColor::parse("#ffffff").unwrap().luminance();
        let dark = HexColor::parse("#0b1018").unwrap().luminance();
        let light = HexColor::parse("#eceff4").unwrap().luminance();
        assert_eq!(black, 0.0);
        assert_eq!(white, 1.0);
        assert!(dark < 0.01, "dark background luminance was {dark}");
        assert!(light > 0.8, "light background luminance was {light}");
    }
}
