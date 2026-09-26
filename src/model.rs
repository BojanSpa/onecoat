//! The domain model: ids, colours, palettes, and the two theme phases.

pub mod color;
pub mod ids;
pub mod palette;
pub mod reserved;
pub mod theme;

pub use color::HexColor;
pub use ids::{Appearance, ThemeId};
pub use palette::Base16Entry;
