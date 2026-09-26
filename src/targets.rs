//! Where onecoat's files live.

use std::env;
use std::path::PathBuf;

use crate::Error;

#[allow(missing_docs)]
pub struct Paths {
    pub wt_fragment: PathBuf,
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
