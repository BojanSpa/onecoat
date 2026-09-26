//! The domain model: ids, colours, palettes, and the two theme phases.
//!
//! This module is the boundary: [`ThemeId`] and [`HexColor`] are parsed here, so no
//! renderer ever sees an unparsed value, and a theme is [parsed](theme::Theme::parse)
//! into a shape that [validation](crate::validate) can consume without touching IO.

pub mod color;
pub mod ids;
pub mod palette;
pub mod reserved;
pub mod theme;

pub use color::HexColor;
pub use ids::{Appearance, ThemeId};
pub use palette::Base16Entry;
