//! RFC 4648 base32 decoding (the TOTP secret).

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Decodes base32 text, ignoring case, `=` padding and whitespace.
///
/// Returns `None` when a character is not base32 or nothing is left after cleaning.
#[must_use]
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut value = 0u32;
    let mut out = Vec::new();
    let mut seen = false;
    for c in text.chars().filter(|c| !c.is_whitespace() && *c != '=') {
        let upper = u8::try_from(c.to_ascii_uppercase()).ok()?;
        let index = ALPHABET.iter().position(|a| *a == upper)?;
        seen = true;
        value = (value << 5) | u32::try_from(index).ok()?;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((value >> bits) & 0xff).ok()?);
        }
        value &= (1 << bits) - 1;
    }
    seen.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_the_rfc_secret_ignoring_case_padding_and_spaces() {
        let raw = b"12345678901234567890".to_vec();

        assert_eq!(
            decode("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"),
            Some(raw.clone())
        );
        assert_eq!(
            decode("gezd gnbv gy3t qojq gezd gnbv gy3t qojq=="),
            Some(raw)
        );
    }

    #[test]
    fn rejects_non_base32_and_empty_text() {
        assert_eq!(decode("GEZ1"), None);
        assert_eq!(decode("  "), None);
        assert_eq!(decode("é"), None);
    }
}
