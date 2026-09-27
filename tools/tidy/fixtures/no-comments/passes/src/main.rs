pub fn slash() -> char {
    '/'
}

pub fn url() -> &'static str {
    "https://example.test//path"
}

pub fn raw() -> &'static str {
    r#"// not a comment"#
}

pub fn lifetime<'a>(text: &'a str) -> &'a str {
    text
}
