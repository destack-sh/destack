use destack_std_unicode::xid::UnicodeXID;

/// Checks if `c` is considered a whitespace according to Unicode `Pattern_White_Space``.
pub fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        // usual ASCII suspects
        '\u{0009}'   // \t
			| '\u{000A}' // \n
			| '\u{000B}' // vertical tab
			| '\u{000C}' // form feed
			| '\u{000D}' // \r
			| '\u{0020}' // space
			// NEXT LINE from latin1
			| '\u{0085}'
			// bidi markers
			| '\u{200E}' // LEFT-TO-RIGHT MARK
			| '\u{200F}' // RIGHT-TO-LEFT MARK
			// dedicated whitespace characters from Unicode
			| '\u{2028}' // LINE SEPARATOR
			| '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

/// Checks if `c` is valid as a first character of an identifier.
pub fn is_id_start(c: char) -> bool {
    c == '_' || UnicodeXID::is_xid_start(c)
}

/// Checks if `c` is valid as a non-first character of an identifier.
pub fn is_id_continue(c: char) -> bool {
    UnicodeXID::is_xid_continue(c)
}

/// Checks if the passed string is lexically an identifier.
pub fn is_ident(string: &str) -> bool {
    let mut chars = string.chars();
    if let Some(start) = chars.next() {
        is_id_start(start) && chars.all(is_id_continue)
    } else {
        false
    }
}
