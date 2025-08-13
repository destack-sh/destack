//! Format utilities for `Uuid`.

#[inline]
const fn to_hex_char(n: u8) -> u8 {
    match n {
        0..=9 => b'0' + n,
        10..=15 => b'a' + (n - 10),
        _ => b'?',
    }
}

/// Format a raw `u128` UUID into the hyphenated canonical form.
///
/// Example: 00112233-4455-6677-8899-aabbccddeeff
#[inline]
pub(crate) fn format_uuid(u: u128) -> [u8; 36] {
    let mut out = [0u8; 36];
    // positions of dashes in the 36-byte buffer
    const DASHES: [usize; 4] = [8, 13, 18, 23];
    let mut i = 35usize;
    let mut j = 0usize; // number of dashes written from the end
    let mut val = u;
    // write from the end for simplicity
    while i < 36 {
        if j < 4 && i == DASHES[3 - j] {
            out[i] = b'-';
            j += 1;
            if i == 0 {
                break;
            }
            i -= 1;
            continue;
        }

        let nibble = (val & 0xF) as u8;
        out[i] = to_hex_char(nibble);
        val >>= 4;
        if i == 0 {
            break;
        }
        i -= 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Format a known value into hyphenated lower.
    fn format_known() {
        let v = 0x00112233445566778899aabbccddeeffu128;
        let s = format_uuid(v);
        assert_eq!(
            std::str::from_utf8(&s).unwrap(),
            "00112233-4455-6677-8899-aabbccddeeff"
        );
    }
}
