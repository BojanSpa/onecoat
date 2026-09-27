use std::{env, path::PathBuf};

pub fn home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}
