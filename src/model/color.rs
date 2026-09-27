use std::fmt;

use serde::{Serialize, Serializer};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HexColor {
    r: u8,
    g: u8,
    b: u8,
}

impl HexColor {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
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

    pub(crate) const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub(crate) fn luminance(self) -> f64 {
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
#[path = "../../tests/unit/model/color.rs"]
mod tests;
