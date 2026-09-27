use super::HexColor;

#[test]
fn parse_accepts_six_hex_digits_in_either_case() {
    assert_eq!(HexColor::parse("#0B1018"), HexColor::parse("#0b1018"));
    assert!(HexColor::parse("#0B1018").is_some());
    assert!(HexColor::parse("#0b1018").is_some());
}

#[test]
fn parse_rejects_shapes_windows_terminal_rejects() {
    for raw in ["#abc", "#0b1018ff", "0b1018", "#gggggg", "#", ""] {
        assert_eq!(HexColor::parse(raw), None, "{raw:?} must not parse");
    }
}

#[test]
fn display_writes_a_lowercase_hash_colour() {
    assert_eq!(HexColor::parse("#0B1018").unwrap().to_string(), "#0b1018");
}

#[test]
fn luminance_spans_the_full_range() {
    let black = HexColor::parse("#000000").unwrap().luminance();
    let white = HexColor::parse("#ffffff").unwrap().luminance();
    let dark = HexColor::parse("#0b1018").unwrap().luminance();
    let light = HexColor::parse("#eceff4").unwrap().luminance();
    assert_eq!(black, 0.0);
    assert_eq!(white, 1.0);
    assert!(dark < 0.01, "dark background luminance was {dark}");
    assert!(light > 0.8, "light background luminance was {light}");
}
