use std::fmt;
use std::io;

mod inner {
    pub const LIMIT: u8 = 3;

    pub fn read() -> u8 {
        LIMIT
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Slot {
    Dark,
    Light,
}

pub struct Item {
    value: u8,
}

impl Item {
    pub(crate) fn value(&self) -> u8 {
        self.value
    }

    #[allow(dead_code)]
    pub fn doubled(&self) -> u8 {
        self.value * 2
    }
}

const SNIPPET: &str = "
}
fn not_an_item() {}
";

fn total_of(limit: u8) -> u8 {
    let mut total = 0;
    if limit > 0 {
        total += 1;
    }
    for step in 0..limit {
        total += step;
    }
    total
}

fn uses_are_left_alone(item: &Item) {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = write!(out, "{}", item.doubled());
    let _ = out;
    let _ = fmt::Arguments::as_str;
    let _ = io::empty;
    let _ = total_of(2);
}
