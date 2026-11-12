//! UTC calendar formatting of epoch milliseconds (no clock: callers pass the instant).

const DAY_MS: i64 = 86_400_000;

/// Days since 1970-01-01 to a civil date (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The UTC calendar day `yyyy-mm-dd` that contains `ms` (the usage-day rule: resets 00:00 UTC).
#[must_use]
pub fn utc_day(ms: i64) -> String {
    let (y, m, d) = civil_from_days(ms.div_euclid(DAY_MS));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Whole seconds from `ms` to the next 00:00 UTC (at least 1), the wait after the daily cap.
#[must_use]
pub fn seconds_until_next_utc_day(ms: i64) -> u64 {
    let left_ms = DAY_MS - ms.rem_euclid(DAY_MS);
    u64::try_from((left_ms + 999) / 1000).unwrap_or(1).max(1)
}

/// ISO-8601 UTC with milliseconds and `Z`, e.g. `2026-11-12T01:00:00.000Z`.
#[must_use]
pub fn iso_instant(ms: i64) -> String {
    let rest = ms.rem_euclid(DAY_MS);
    let (h, min, s, milli) = (
        rest / 3_600_000,
        rest / 60_000 % 60,
        rest / 1000 % 60,
        rest % 1000,
    );
    format!("{}T{h:02}:{min:02}:{s:02}.{milli:03}Z", utc_day(ms))
}

/// Parses the exact `iso_instant` format back to epoch ms. `None` for anything else.
#[must_use]
pub fn parse_iso_instant(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    let shape = b.len() == 24 && b[4] == b'-' && b[7] == b'-' && b[10] == b'T';
    if !shape || b[13] != b':' || b[16] != b':' || b[19] != b'.' || b[23] != b'Z' {
        return None;
    }
    let num = |from: usize, to: usize| text.get(from..to)?.parse::<i64>().ok();
    let (y, m, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let days = days_from_civil(y, m, d);
    let ms = days * DAY_MS
        + num(11, 13)? * 3_600_000
        + num(14, 16)? * 60_000
        + num(17, 19)? * 1000
        + num(20, 23)?;
    (iso_instant(ms) == text).then_some(ms)
}

/// A civil date to days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::{iso_instant, parse_iso_instant, utc_day};
    use proptest::prelude::*;

    #[test]
    fn formats_known_instants() {
        assert_eq!(iso_instant(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_instant(1_794_445_200_000), "2026-11-12T01:00:00.000Z");
        assert_eq!(iso_instant(951_782_400_123), "2000-02-29T00:00:00.123Z");
        assert_eq!(utc_day(1_794_527_999_999), "2026-11-12");
        assert_eq!(utc_day(1_794_528_000_000), "2026-11-13");
    }

    #[test]
    fn rejects_other_formats() {
        assert_eq!(parse_iso_instant("2026-11-12T01:00:00Z"), None);
        assert_eq!(parse_iso_instant("2026-02-30T00:00:00.000Z"), None);
    }

    proptest! {
        #[test]
        fn parse_inverts_format(ms in 0i64..4_102_444_800_000) {
            prop_assert_eq!(parse_iso_instant(&iso_instant(ms)), Some(ms));
        }
    }
}
