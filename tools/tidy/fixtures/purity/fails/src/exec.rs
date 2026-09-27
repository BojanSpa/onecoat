pub fn write(path: &str) -> std::io::Result<()> {
    std::fs::write(path, "")
}
