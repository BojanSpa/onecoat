pub fn digits_of(id: u64) -> String {
    let digits = id.to_string();

    if digits.is_empty() {
        return "zero".to_owned();
    }

    digits
}
