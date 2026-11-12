//! Id formats (same as `backend/src/adapters/crypto-id-generator/v1.ts`): a 16-hex id is 12 hex of
//! epoch ms plus a 4-hex tail, so ids sort by creation; a message id is a ULID. The randomness and
//! the monotonic state live in the `IdGenerator` adapter; these are the pure encodings.

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Random characters in a ULID.
pub const ULID_RANDOM_CHARS: usize = 16;

/// The 16-hex id for `ms` and a 16-bit tail: the low 12 hex digits of `ms`, then the tail.
#[must_use]
pub fn format_id(ms: i64, tail: u16) -> String {
    let millis = format!("{ms:012x}");
    format!("{}{tail:04x}", &millis[millis.len() - 12..])
}

/// The ULID for `ms` and 16 base-32 digits (each 0..32, most significant first).
#[must_use]
pub fn format_ulid(ms: i64, digits: &[u8; ULID_RANDOM_CHARS]) -> String {
    let mut time = [0u8; 10];
    let mut rest = ms;
    for slot in time.iter_mut().rev() {
        *slot = CROCKFORD[usize::try_from(rest.rem_euclid(32)).unwrap_or(0)];
        rest = rest.div_euclid(32);
    }
    let random = digits.iter().map(|d| CROCKFORD[usize::from(d % 32)]);
    time.iter().copied().chain(random).map(char::from).collect()
}

/// Adds one to base-32 digits (most significant first), wrapping on overflow.
#[must_use]
pub fn increment(digits: &[u8; ULID_RANDOM_CHARS]) -> [u8; ULID_RANDOM_CHARS] {
    let mut out = *digits;
    for d in out.iter_mut().rev() {
        let next = *d + 1;
        *d = next % 32;
        if next < 32 {
            break;
        }
    }
    out
}

/// Whether `text` is a 26-character Crockford ULID.
#[must_use]
pub fn is_ulid(text: &str) -> bool {
    text.len() == 26 && text.bytes().all(|b| CROCKFORD.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::{format_id, format_ulid, increment, is_ulid};

    #[test]
    fn formats_a_time_ordered_hex_id() {
        let id = format_id(1_794_445_200_000, 0x002a);

        assert_eq!(id, "01a1cd44ca80002a");
        assert!(format_id(1_794_445_200_001, 0) > id);
    }

    #[test]
    fn formats_a_ulid_with_the_time_first() {
        let ulid = format_ulid(1_794_445_200_000, &[0; 16]);

        assert_eq!(ulid.len(), 26);
        assert!(is_ulid(&ulid));
        assert!(ulid.ends_with("0000000000000000"));
        assert!(format_ulid(1_794_445_200_001, &[0; 16]) > ulid);
    }

    #[test]
    fn increments_with_carry() {
        let mut digits = [0u8; 16];
        digits[15] = 31;

        let next = increment(&digits);

        assert_eq!(next[15], 0);
        assert_eq!(next[14], 1);
    }
}
