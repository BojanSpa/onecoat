//! Pure renderers: a validated theme in, data out.
//!
//! Renderers never touch paths, files, or processes (N-5); they return the values a
//! writer needs, and the writer decides where they go.

pub mod wt;
