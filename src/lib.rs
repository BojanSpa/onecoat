//! onecoat applies one canonical theme to Windows Terminal, Herdr, and the omp harness,
//! so all three look like a single surface when they sit on screen together.
//!
//! The crate is one pipeline:
//!
//! - [`model`] parses ids and colours at the boundary, so nothing downstream ever sees
//!   an unparsed value, and [`validate`] turns a [`model::theme::Theme<Parsed>`] into a
//!   [`model::theme::Theme<Validated>`].
//! - Renderers such as [`render::wt`] are pure functions from a validated theme to data;
//!   they never touch paths, files, or processes.
//! - [`plan::Plan`] is the pure description of the writes an apply needs, and
//!   [`exec::execute`] is the only code in the crate that touches the filesystem.
//!
//! Every failure is a single [`Error`] whose message names the file, the key, and the
//! action that resolves it (R-42).

#![deny(missing_docs)]

pub mod error;
pub mod exec;
pub mod model;
pub mod plan;
pub mod render;
pub mod rolemap;
pub mod targets;
pub mod themes;
pub mod validate;

pub use error::Error;
