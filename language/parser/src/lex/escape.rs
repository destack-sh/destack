use std::ops::RangeInclusive;
use std::str::Chars;

/// The code units spelling a high surrogate.
const HIGH_SURROGATES: RangeInclusive<u32> = 0xD800..=0xDBFF;
/// The code units spelling a low surrogate.
const LOW_SURROGATES: RangeInclusive<u32> = 0xDC00..=0xDFFF;

/// One rejected escape sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidEscape;

/// Decode every escape sequence in one literal body.
pub fn cook(content: &str) -> Result<String, InvalidEscape> {
    let mut cooked = String::with_capacity(content.len());
    let mut characters = content.chars();
    while let Some(character) = characters.next() {
        // copy plain text through
        if character != '\\' {
            cooked.push(character);
            continue;
        }

        // decode the escape, a line continuation adds nothing
        if let Some(character) = decode_escape(&mut characters)? {
            cooked.push(character);
        }
    }

    Ok(cooked)
}

/// Decode one escape sequence after its backslash.
///
/// A line continuation decodes to no character.
pub fn decode_escape(characters: &mut Chars<'_>) -> Result<Option<char>, InvalidEscape> {
    let escaped = characters.next().ok_or(InvalidEscape)?;
    let decoded = match escaped {
        // join line continuations
        '\n' | '\u{2028}' | '\u{2029}' => None,
        '\r' => {
            if characters.clone().next() == Some('\n') {
                characters.next();
            }
            None
        }
        // allow the null escape alone, never a legacy octal escape
        '0' if !characters
            .clone()
            .next()
            .is_some_and(|next| next.is_ascii_digit()) =>
        {
            Some('\0')
        }
        '0'..='9' => return Err(InvalidEscape),
        'b' => Some('\u{08}'),
        'f' => Some('\u{0C}'),
        'n' => Some('\n'),
        'r' => Some('\r'),
        't' => Some('\t'),
        'v' => Some('\u{0B}'),
        'x' => Some(decode_hex_escape(characters, 2)?),
        'u' => Some(decode_unicode_escape(characters)?),
        character => Some(character),
    };

    Ok(decoded)
}

/// Decode one `\u` escape body, either four hex digits or a braced code point.
///
/// A high surrogate escape joins the low surrogate escape that follows it.
pub fn decode_unicode_escape(characters: &mut Chars<'_>) -> Result<char, InvalidEscape> {
    let value = decode_unicode_value(characters)?;

    // join a surrogate pair spelled as two escapes
    if (HIGH_SURROGATES).contains(&value) {
        let mut rest = characters.clone();
        if rest.next() == Some('\\') && rest.next() == Some('u') {
            let low = decode_unicode_value(&mut rest)?;
            if (LOW_SURROGATES).contains(&low) {
                *characters = rest;
                let joined = 0x10000
                    + ((value - HIGH_SURROGATES.start()) << 10)
                    + (low - LOW_SURROGATES.start());

                return char::from_u32(joined).ok_or(InvalidEscape);
            }
        }
    }

    char::from_u32(value).ok_or(InvalidEscape)
}

/// Decode the numeric value of one `\u` escape body.
fn decode_unicode_value(characters: &mut Chars<'_>) -> Result<u32, InvalidEscape> {
    if characters.clone().next() != Some('{') {
        return decode_hex_value(characters, 4);
    }
    characters.next();

    // read hex digits up to the closing brace, rejecting values past the code space
    let mut value = 0u32;
    let mut digits = 0usize;
    loop {
        match characters.next() {
            Some('}') if digits > 0 => break,
            Some(digit) if digit.is_ascii_hexdigit() => {
                let digit = digit.to_digit(16).ok_or(InvalidEscape)?;
                value = value
                    .checked_mul(16)
                    .and_then(|value| value.checked_add(digit))
                    .filter(|value| *value <= char::MAX as u32)
                    .ok_or(InvalidEscape)?;
                digits += 1;
            }
            _ => return Err(InvalidEscape),
        }
    }

    Ok(value)
}

/// Decode an exact number of hex digits into one character.
fn decode_hex_escape(characters: &mut Chars<'_>, width: usize) -> Result<char, InvalidEscape> {
    let value = decode_hex_value(characters, width)?;

    char::from_u32(value).ok_or(InvalidEscape)
}

/// Decode an exact number of hex digits into their value.
fn decode_hex_value(characters: &mut Chars<'_>, width: usize) -> Result<u32, InvalidEscape> {
    let mut value = 0u32;
    for _ in 0..width {
        let digit = characters.next().and_then(|digit| digit.to_digit(16));
        value = value * 16 + digit.ok_or(InvalidEscape)?;
    }

    Ok(value)
}
