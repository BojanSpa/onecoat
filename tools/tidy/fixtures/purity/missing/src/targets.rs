use std::env;

pub fn local() -> Option<String> {
    env::var("LOCALAPPDATA").ok()
}
