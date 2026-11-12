//! Text measures that match the TypeScript contract (string lengths are UTF-16 units there),
//! and a constant-time byte comparison.

/// Length in UTF-16 code units (JavaScript `string.length`).
#[must_use]
pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The longest prefix of `text` whose UTF-16 length is at most `max`, never splitting a character.
#[must_use]
pub fn truncate_utf16(text: &str, max: usize) -> &str {
    let mut used = 0;
    for (index, c) in text.char_indices() {
        used += c.len_utf16();
        if used > max {
            return &text[..index];
        }
    }
    text
}

/// Whether two byte strings are equal, taking the same time for any content of equal length.
#[must_use]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Whether `text` is a 16-lowercase-hex id.
#[must_use]
pub fn is_hex_id(text: &str) -> bool {
    text.len() == 16
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::{constant_time_eq, is_hex_id, truncate_utf16, utf16_len};

    #[test]
    fn measures_and_truncates_in_utf16_units() {
        assert_eq!(utf16_len("a😀"), 3);
        assert_eq!(truncate_utf16("ab😀c", 3), "ab");
        assert_eq!(truncate_utf16("ab😀c", 4), "ab😀");
        assert_eq!(truncate_utf16("abc", 10), "abc");
    }

    #[test]
    fn compares_bytes_by_content_and_length() {
        assert!(constant_time_eq(b"123456", b"123456"));
        assert!(!constant_time_eq(b"123456", b"123457"));
        assert!(!constant_time_eq(b"12345", b"123456"));
    }

    #[test]
    fn accepts_only_sixteen_lowercase_hex() {
        assert!(is_hex_id("0123456789abcdef"));
        assert!(!is_hex_id("0123456789ABCDEF"));
        assert!(!is_hex_id("0123456789abcde"));
    }
}
