use std::fmt;
fn first() -> u8 {
    1
}
fn second() -> u8 {
    2
}

pub struct Item {
    value: u8,
}
impl Item {
    fn value(&self) -> u8 {
        self.value
    }
}
pub(crate) fn third() -> u8 {
    3
}
#[allow(dead_code)]
fn fourth() -> u8 {
    4
}
