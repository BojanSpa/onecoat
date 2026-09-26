//! Path resolution: the platform edge (N-5).
//!
//! Both roots are resolved for every command, so there is exactly one path-resolution
//! path and a missing environment variable is reported the same way everywhere.

use std::env;
use std::path::PathBuf;

use crate::Error;

/// The paths onecoat reads and writes.
pub struct Paths {
    /// `%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\onecoat\schemes.json`.
    pub wt_fragment: PathBuf,
    /// `%APPDATA%\onecoat\themes`.
    pub user_themes: PathBuf,
}

impl Paths {
    #[allow(missing_docs)]
    pub fn resolve() -> Result<Self, Error> {
        let local = env::var_os("LOCALAPPDATA").ok_or(Error::EnvMissing {
            var: "LOCALAPPDATA",
        })?;
        let roaming = env::var_os("APPDATA").ok_or(Error::EnvMissing { var: "APPDATA" })?;
        Ok(Self {
            wt_fragment: PathBuf::from(local)
                .join("Microsoft")
                .join("Windows Terminal")
                .join("Fragments")
                .join("onecoat")
                .join("schemes.json"),
            user_themes: PathBuf::from(roaming).join("onecoat").join("themes"),
        })
    }
}
