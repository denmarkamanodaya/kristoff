//! Roman numerals for article numbers (`Art. III`). Pure; property-tested in `tests/reference.rs`.

const TABLE: [(u32, &str); 13] = [
    (1000, "m"),
    (900, "cm"),
    (500, "d"),
    (400, "cd"),
    (100, "c"),
    (90, "xc"),
    (50, "l"),
    (40, "xl"),
    (10, "x"),
    (9, "ix"),
    (5, "v"),
    (4, "iv"),
    (1, "i"),
];

/// Formats `n` (1–3999) as a lower-case Roman numeral; `None` outside that range.
#[must_use]
pub fn to_roman(mut n: u32) -> Option<String> {
    if !(1..=3999).contains(&n) {
        return None;
    }
    let mut out = String::new();
    for (value, digits) in TABLE {
        while n >= value {
            out.push_str(digits);
            n -= value;
        }
    }
    Some(out)
}

/// Parses a canonical Roman numeral (any case) to 1–3999; `None` when not canonical.
///
/// Canonical means it round-trips through [`to_roman`], so `iiii` and `ic` are rejected.
#[must_use]
pub fn from_roman(text: &str) -> Option<u32> {
    let lower = text.to_ascii_lowercase();
    if lower.is_empty() || lower.len() > 15 {
        return None;
    }
    let mut rest = lower.as_str();
    let mut total = 0u32;
    for (value, digits) in TABLE {
        while let Some(tail) = rest.strip_prefix(digits) {
            total += value;
            rest = tail;
        }
    }
    let canonical = rest.is_empty() && to_roman(total).is_some_and(|r| r == lower);
    canonical.then_some(total)
}
