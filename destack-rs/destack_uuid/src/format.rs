//! Format utilities for `Uuid`.

#[inline]
const fn to_hex_char(n: u8) -> u8 {
    match n {
        0..=9 => b'0' + n,
        10..=15 => b'a' + (n - 10),
        _ => b'?',
    }
}

/// Format a `[u8; 16]` UUID into the hyphenated canonical form.
///
/// Example: 00112233-4455-6677-8899-aabbccddeeff
#[inline]
pub(crate) fn format_uuid(bytes: [u8; 16]) -> [u8; 36] {
    let mut out = [0u8; 36];
    let mut out_idx = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        // add dashes at positions 4, 6, 8, 10 (after bytes 3, 5, 7, 9)
        if i == 4 || i == 6 || i == 8 || i == 10 {
            out[out_idx] = b'-';
            out_idx += 1;
        }

        // high nibble
        out[out_idx] = to_hex_char((byte >> 4) & 0xF);
        out_idx += 1;
        // low nibble
        out[out_idx] = to_hex_char(byte & 0xF);
        out_idx += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Format a known value into hyphenated lower.
    fn format_known() {
        let bytes = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let s = format_uuid(bytes);
        assert_eq!(
            std::str::from_utf8(&s).unwrap(),
            "00112233-4455-6677-8899-aabbccddeeff"
        );
    }
}
