use std::path::Path;

pub fn splice(path: &Path, text: &str) -> String {
    let _ = path;
    text.to_owned()
}
