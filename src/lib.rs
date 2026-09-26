//! onecoat applies one canonical theme to Windows Terminal, Herdr, and the omp harness.

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
