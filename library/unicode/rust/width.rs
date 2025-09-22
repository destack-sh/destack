use crate::general_category::{GeneralCategory, UnicodeGeneralCategory};

/// Compute the terminal display width of Unicode scalars.
///
/// The implementation is intentionally lightweight. It follows the general
/// [`wcwidth`](https://www.cl.cam.ac.uk/%7Emgk25/ucs/wcwidth.c) approach of
/// returning `0` for control and combining code points, `1` for regular
/// characters, and `2` for characters that are commonly rendered with two
/// columns in monospace terminals (CJK ideographs, Hangul syllables,
/// compatibility forms, and emoji presentation characters).
///
/// The data tables are derived from the Unicode Standard and mirror the ranges
/// that modern terminals treat as wide. They deliberately avoid the ambiguous
/// East Asian width ranges, which remain single width.
pub trait UnicodeWidthChar {
    /// Return the estimated terminal column width of this scalar.
    fn terminal_display_width(self) -> u8;
}

impl UnicodeWidthChar for char {
    #[inline]
    fn terminal_display_width(self) -> u8 {
        if is_zero_width(self) {
            return 0;
        } else if is_wide(self) {
            return 2;
        } else {
            1
        }
    }
}

fn is_zero_width(c: char) -> bool {
    matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}'..='\u{009F}')
        || matches!(c, '\u{2060}'..='\u{2064}' | '\u{206A}'..='\u{206F}')
        || matches!(c, '\u{FE00}'..='\u{FE0F}')
        || matches!(c, '\u{E0000}'..='\u{E0FFF}')
        || crate::emoji::is_zwj(c)
        || matches!(
            c.general_category(),
            GeneralCategory::Control
                | GeneralCategory::Format
                | GeneralCategory::NonspacingMark
                | GeneralCategory::EnclosingMark
        )
}

fn is_wide(c: char) -> bool {
    let code = c as u32;

    for &(start, end) in WIDE_RANGES {
        if code < start {
            return false;
        }

        if code <= end {
            return true;
        }
    }

    false
}

// NOTE @Cleanup: keep this list in sync with Unicode east Asian wide ranges when upgrading unicode tables
const WIDE_RANGES: &[(u32, u32)] = &[
    (0x1100, 0x115F),
    (0x231A, 0x231B),
    (0x2329, 0x232A),
    (0x23E9, 0x23EC),
    (0x23F0, 0x23F0),
    (0x23F3, 0x23F3),
    (0x25FD, 0x25FE),
    (0x2614, 0x2615),
    (0x2648, 0x2653),
    (0x267F, 0x267F),
    (0x2693, 0x2693),
    (0x26A1, 0x26A1),
    (0x26AA, 0x26AB),
    (0x26BD, 0x26BE),
    (0x26C4, 0x26C5),
    (0x26CE, 0x26CE),
    (0x26D4, 0x26D4),
    (0x26EA, 0x26EA),
    (0x26F2, 0x26F3),
    (0x26F5, 0x26F5),
    (0x26FA, 0x26FA),
    (0x26FD, 0x26FD),
    (0x2705, 0x2705),
    (0x270A, 0x270B),
    (0x2728, 0x2728),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2795, 0x2797),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x2E80, 0x2FFB),
    (0x3000, 0x303E),
    (0x3041, 0x33FF),
    (0x3400, 0x4DBF),
    (0x4E00, 0xA4C6),
    (0xA960, 0xA97C),
    (0xAC00, 0xD7A3),
    (0xF900, 0xFAFF),
    (0xFE10, 0xFE19),
    (0xFE30, 0xFE6B),
    (0xFF01, 0xFF60),
    (0xFFE0, 0xFFE6),
    (0x16FE0, 0x16FE3),
    (0x17000, 0x187F7),
    (0x18800, 0x18CD5),
    (0x18D00, 0x18D08),
    (0x1B000, 0x1B11E),
    (0x1B150, 0x1B152),
    (0x1B164, 0x1B167),
    (0x1B170, 0x1B2FB),
    (0x1F004, 0x1F0CF),
    (0x1F100, 0x1F27F),
    (0x1F300, 0x1F4FC),
    (0x1F500, 0x1F6FF),
    (0x1F700, 0x1F773),
    (0x1F780, 0x1F7FF),
    (0x1F800, 0x1F8FF),
    (0x1F900, 0x1FAFF),
    (0x1FC00, 0x1FFFD),
];
