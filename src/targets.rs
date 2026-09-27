use std::env;
use std::path::PathBuf;

use crate::Error;

pub struct Paths {
    pub wt_fragment: PathBuf,
    pub wt_settings: PathBuf,
    pub user_themes: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self, Error> {
        let local = env::var_os("LOCALAPPDATA").ok_or(Error::EnvMissing {
            var: "LOCALAPPDATA",
        })?;
        let roaming = env::var_os("APPDATA").ok_or(Error::EnvMissing { var: "APPDATA" })?;
        let local = PathBuf::from(local);
        Ok(Self {
            wt_fragment: local
                .join("Microsoft")
                .join("Windows Terminal")
                .join("Fragments")
                .join("onecoat")
                .join("schemes.json"),
            wt_settings: local
                .join("Packages")
                .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
                .join("LocalState")
                .join("settings.json"),
            user_themes: PathBuf::from(roaming).join("onecoat").join("themes"),
        })
    }
}
