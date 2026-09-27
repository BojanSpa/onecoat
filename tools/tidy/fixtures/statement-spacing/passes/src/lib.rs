pub fn digits(id: u64) -> String {
    let digits = id.to_string();
    if digits.is_empty() {
        return "zero".to_owned();
    }

    digits
}

pub fn total_of(plan: &[u8]) -> usize {
    let mut total = 0;
    for step in plan {
        total += usize::from(*step);
    }

    total
}

pub fn label(
    id: u64,
    plan: &[u8],
) -> String {
    let total = total_of(plan);
    match total {
        0 => format!("{id}: empty"),
        _ => format!("{id}: {total}"),
    }
}

pub fn quoted(flag: bool) -> char {
    let quote = '"';
    if flag {
        return quote;
    }

    match quote {
        'x' => 'x',
        _ => quote,
    }
}

pub struct Item {
    pub value: u8,
}

pub fn built(limit: u8) -> Item {
    let mut value = 0;
    if limit > 0 {
        value = limit;
    }

    Item { value }
}

pub fn doubled(
    plan: &[u8],
) -> usize {
    let total = total_of(plan);

    let doubled = total
        .saturating_mul(2);

    doubled
}

pub fn size_of(plan: &[u8]) -> &'static str {
    let total = total_of(plan);
    if total == 0 {
        "empty"
    } else if total < 10 {
        "small"
    } else {
        "large"
    }
}
