pub fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}
