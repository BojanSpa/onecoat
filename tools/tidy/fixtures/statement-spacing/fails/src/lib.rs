pub fn digits(id: u64) -> String {
    if id == 0 {
        return "zero".to_owned();
    }
    id.to_string()
}

pub fn total_of(plan: &[u8]) -> usize {
    let mut total = 0;
    for step in plan {
        total += usize::from(*step);
    }
    total
}

pub fn picked(
    limit: u8,
    plan: &[u8],
) -> usize {
    let mut total = 0;
    for step in plan.iter().take(usize::from(limit)) {
        total += usize::from(*step);
    }
    total
}

pub fn counted(plan: &[u8]) -> usize {
    let mut total = 0;
    for step in plan {
        total += usize::from(*step);
    }
    let doubled = total
        .saturating_mul(2);
    doubled
}
