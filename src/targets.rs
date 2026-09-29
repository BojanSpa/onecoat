use std::env;
use std::path::{Path, PathBuf};

use crate::Error;
use crate::model::ids::Slot;
use crate::render::omp;

const AGENT_DIR_VAR: &str = "PI_CODING_AGENT_DIR";
const HERDR_CONFIG_VAR: &str = "HERDR_CONFIG_PATH";
const HOME_VAR: &str = "USERPROFILE";

pub const HERDR_PROGRAM: &str = "herdr";

pub struct Paths {
    pub wt_fragment: PathBuf,
    pub wt_settings: PathBuf,
    pub herdr_config: PathBuf,
    pub herdr_socket: PathBuf,
    pub omp_themes: PathBuf,
    pub omp_config: PathBuf,
    pub user_themes: PathBuf,
    pub state: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self, Error> {
        let local = env::var_os("LOCALAPPDATA").ok_or(Error::EnvMissing {
            var: "LOCALAPPDATA",
        })?;

        let roaming = env::var_os("APPDATA").ok_or(Error::EnvMissing { var: "APPDATA" })?;
        let agent = agent_dir()?;
        let local = PathBuf::from(local);
        let roaming = PathBuf::from(roaming);

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
            herdr_config: herdr_config(&roaming),
            herdr_socket: roaming.join("herdr").join("herdr.sock"),
            omp_themes: agent.join("themes"),
            omp_config: agent.join("config.yml"),
            user_themes: roaming.join("onecoat").join("themes"),
            state: roaming.join("onecoat").join("state.json"),
        })
    }

    pub fn omp_theme(&self, slot: Slot) -> PathBuf {
        self.omp_themes.join(omp::file_name(slot))
    }
}

fn herdr_config(roaming: &Path) -> PathBuf {
    env::var_os(HERDR_CONFIG_VAR)
        .filter(|path| !path.is_empty())
        .map_or_else(|| roaming.join("herdr").join("config.toml"), PathBuf::from)
}

fn agent_dir() -> Result<PathBuf, Error> {
    if let Some(dir) = env::var_os(AGENT_DIR_VAR).filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }

    let home = env::var_os(HOME_VAR).ok_or(Error::EnvMissing { var: HOME_VAR })?;

    Ok(PathBuf::from(home).join(".omp").join("agent"))
}
