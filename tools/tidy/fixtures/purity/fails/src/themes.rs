pub fn appdata() -> Option<std::ffi::OsString> {
    std::env::var_os("APPDATA")
}
