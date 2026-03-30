use std::fmt;

/// Return one escaped debug string.
pub(crate) fn to_escaped_string<T: fmt::Debug>(value: &T) -> String {
    // escaped debug text
    let debug_representation = format!("{value:?}");

    debug_representation
        .chars()
        .flat_map(|character| character.escape_default())
        .collect()
}

/// Return one lowercase ASCII letter when possible.
pub(crate) fn lower_ascii_letter(character: char) -> Option<char> {
    match character {
        'a'..='z' => Some(character),
        'A'..='Z' => Some((character as u8 - b'A' + b'a') as char),
        _ => None,
    }
}
