use super::{AnsiSlot, Base16Entry};

#[test]
fn base16_entries_resolve_case_insensitively() {
    assert_eq!(Base16Entry::from_name("base0a"), Some(Base16Entry::B0A));
    assert_eq!(Base16Entry::from_name("base0A"), Some(Base16Entry::B0A));
    assert_eq!(Base16Entry::from_name("base00"), Some(Base16Entry::B00));
    assert_eq!(Base16Entry::from_name("base10"), None);
    assert_eq!(Base16Entry::from_name("base"), None);
}

#[test]
fn ansi_slots_use_the_windows_terminal_spelling() {
    assert_eq!(AnsiSlot::from_name("purple"), Some(AnsiSlot::Purple));

    assert_eq!(
        AnsiSlot::from_name("brightPurple"),
        Some(AnsiSlot::BrightPurple)
    );

    assert_eq!(AnsiSlot::from_name("magenta"), None);
    assert_eq!(AnsiSlot::from_name("Purple"), None);
}
