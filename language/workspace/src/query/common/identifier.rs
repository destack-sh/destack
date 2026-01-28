use destack_source::{FileContent, FileId};

use crate::Session;

/// Check whether a character can start an identifier.
pub(crate) fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

/// Check whether a character can continue an identifier.
pub(crate) fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

/// Check whether a byte is a simple identifier character.
pub(crate) fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

/// Extract the identifier that contains the given offset.
pub(crate) fn identifier_at_offset(source: &str, offset: u32) -> Option<String> {
    // guard against out of bounds offsets
    let offset = offset as usize;
    if offset > source.len() {
        return None;
    }

    let bytes = source.as_bytes();
    let mut start = offset;
    let mut end = offset;

    // walk left to the start of the identifier
    while start > 0 {
        let byte = bytes[start.saturating_sub(1)] as char;
        if is_identifier_continue(byte) {
            start = start.saturating_sub(1);
        } else {
            break;
        }
    }

    // walk right to the end of the identifier
    while end < bytes.len() {
        let byte = bytes[end] as char;
        if is_identifier_continue(byte) {
            end = end.saturating_add(1);
        } else {
            break;
        }
    }

    let name = source.get(start..end)?;
    if !is_simple_identifier(name) {
        return None;
    }

    Some(name.to_string())
}

/// Extract the identifier token at a given offset.
pub(crate) fn token_at_offset(session: &Session, file_id: FileId, offset: u32) -> Option<String> {
    // read the source content
    let file = session.files.get(file_id);
    let content = match &file.content {
        FileContent::Text { content } => content.as_str(),
        FileContent::Json { content, .. } => content.as_str(),
        _ => return None,
    };

    // clamp the offset to the file length
    let bytes = content.as_bytes();
    let mut index = offset as usize;
    if index >= bytes.len() {
        index = bytes.len().saturating_sub(1);
    }

    // require the cursor to be on an identifier byte
    let current = *bytes.get(index)?;
    if !is_identifier_byte(current) {
        return None;
    }

    // scan left to the start of the token
    let mut start = index;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }

    // scan right to the end of the token
    let mut end = index + 1;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }

    let slice = content.get(start..end)?;
    Some(slice.to_string())
}

/// Extract the first identifier from a string.
pub(crate) fn extract_identifier(text: &str) -> Option<String> {
    // scan for the first valid identifier start
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.peek().copied() {
        if is_identifier_start(ch) {
            break;
        }
        chars.next();
    }

    // collect identifier characters
    let mut name = String::new();
    while let Some(ch) = chars.peek().copied() {
        if name.is_empty() {
            if !is_identifier_start(ch) {
                chars.next();
                continue;
            }
            name.push(ch);
            chars.next();
            continue;
        }

        if is_identifier_continue(ch) {
            name.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    if name.is_empty() {
        return None;
    }

    // return the extracted identifier
    Some(name)
}

/// Check if a string is a simple identifier.
pub(crate) fn is_simple_identifier(text: &str) -> bool {
    // check the first character
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    // validate the first character
    let first_ok = is_identifier_start(first);
    if !first_ok {
        return false;
    }

    // ensure the rest are valid identifier characters
    chars.all(is_identifier_continue)
}
