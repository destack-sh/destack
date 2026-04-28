use std::borrow::Cow;

/// Placeholder prefix for protecting `{@link ...}` tokens from markdown parsing.
/// Uses a format that `tokenize_words` won't split (no spaces, looks like a word).
pub(super) const PLACEHOLDER_PREFIX: &str = "\x00JDLNK";

/// Normalize the legacy `1- foo` list marker style used by some existing Jsdoc
/// fixtures into standard ordered-list syntax so markdown parsing can treat them
/// as list items.
pub(super) fn normalize_legacy_ordered_list_markers(text: &str) -> Cow<'_, str> {
    // fast path
    let bytes = text.as_bytes();
    let has_digit_dash = bytes
        .windows(2)
        .any(|w| w[0].is_ascii_digit() && w[1] == b'-');
    if !has_digit_dash {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut changed = false;

    for line in text.lines() {
        if !result.is_empty() {
            result.push('\n');
        }

        let trimmed = line.trim_start();
        let leading = line.len() - trimmed.len();

        if let Some(first) = trimmed.chars().next()
            && first.is_ascii_digit()
            && let Some(dash_pos) = trimmed.find('-')
            && dash_pos < 5
        {
            let number = &trimmed[..dash_pos];
            if number.chars().all(|c| c.is_ascii_digit()) {
                // normalize numbered list markers written with dashes
                let after_dash = trimmed.as_bytes().get(dash_pos + 1).copied();
                if matches!(after_dash, Some(b' ' | b'\t' | b'|')) {
                    let rest = trimmed[dash_pos + 1..].trim_start();
                    if !rest.is_empty() {
                        result.push_str(&line[..leading]);
                        result.push_str(number);
                        result.push_str(". ");
                        result.push_str(rest);
                        changed = true;
                        continue;
                    }
                }
            }
        }

        result.push_str(line);
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(text)
    }
}

/// Convert `* ` list markers at the start of lines to `- ` to prevent the markdown
/// parser from treating them as emphasis markers. In CommonMark, `* text` after a
/// paragraph is emphasis (italic), not a list item. Converting to `- ` makes the
/// markdown parser correctly recognize these as unordered list items.
pub(super) fn convert_star_list_markers(text: &str) -> Cow<'_, str> {
    if !text.contains("* ") {
        return Cow::Borrowed(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut changed = false;

    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        let trimmed = line.trim_start();
        if let Some(after_star) = trimmed.strip_prefix("* ") {
            let leading = line.len() - trimmed.len();
            result.push_str(&line[..leading]);
            result.push_str("- ");
            result.push_str(after_star);
            changed = true;
        } else {
            result.push_str(line);
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(text)
    }
}

/// Escape `+ ` at the start of continuation lines (lines preceded by a non-empty line)
/// to prevent the markdown parser from treating them as unordered list markers.
/// This handles cases like `min\n+ spacing` in Jsdoc where `+` is an arithmetic operator.
pub(super) fn escape_false_list_markers(text: &str) -> Cow<'_, str> {
    // fast path
    if !text.contains("+ ") {
        return Cow::Borrowed(text);
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::with_capacity(text.len());
    let mut changed = false;

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }

        let trimmed = line.trim_start();

        // detect real list continuation
        let prev_is_list_item = i > 0 && {
            let prev = lines[i - 1].trim_start();
            prev.starts_with("+ ")
                || prev.starts_with("- ")
                || prev.starts_with("* ")
                || prev
                    .strip_prefix(|c: char| c.is_ascii_digit())
                    .is_some_and(|r| {
                        r.trim_start_matches(|c: char| c.is_ascii_digit())
                            .starts_with(". ")
                    })
        };
        if trimmed.starts_with("+ ")
            && i > 0
            && !lines[i - 1].trim().is_empty()
            && !prev_is_list_item
        {
            let leading = line.len() - trimmed.len();

            result.push_str(&line[..leading]);
            result.push_str("\\+ ");
            result.push_str(&trimmed[2..]);
            changed = true;
        } else {
            result.push_str(line);
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(text)
    }
}

/// Replace `{@link ...}`, `{@linkcode ...}`, `{@linkplain ...}`, `{@tutorial ...}`
/// with numbered placeholders so the markdown parser (especially GFM autolink) doesn't
/// mangle URLs inside them.
pub(super) fn protect_jsdoc_links(text: &str) -> (Cow<'_, str>, Vec<&str>) {
    // fast path
    if !text.contains("{@") {
        return (Cow::Borrowed(text), Vec::new());
    }

    let mut result = String::with_capacity(text.len());
    let mut placeholders = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'{' && i + 1 < len && bytes[i + 1] == b'@' {
            let start = i;
            let mut depth = 1;
            i += 2;
            while i < len && depth > 0 {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }

            // include attached punctuation
            while i < len && matches!(bytes[i], b'.' | b',' | b';' | b':' | b'!' | b'?') {
                i += 1;
            }

            let token = &text[start..i];
            let idx = placeholders.len();
            placeholders.push(token);

            // preserve token width for wrapping
            let idx_str = idx.to_string();
            let placeholder_len = PLACEHOLDER_PREFIX.len() + idx_str.len();
            result.push_str(PLACEHOLDER_PREFIX);
            result.push_str(&idx_str);

            // pad placeholder
            for _ in placeholder_len..token.len() {
                result.push('\x01');
            }
        } else {
            let Some(ch) = text[i..].chars().next() else {
                break;
            };

            result.push(ch);
            i += ch.len_utf8();
        }
    }

    (Cow::Owned(result), placeholders)
}

/// Restore all placeholder tokens in a string back to their original `{@link ...}` form.
pub(super) fn restore_in_string<'a>(s: &'a str, placeholders: &[&str]) -> Cow<'a, str> {
    if placeholders.is_empty() || !s.contains(PLACEHOLDER_PREFIX) {
        return Cow::Borrowed(s);
    }

    Cow::Owned(replace_placeholders(s, placeholders))
}

/// Single-pass scan that replaces all `PLACEHOLDER_PREFIX<digits>` occurrences
/// with their original strings from `placeholders`.
fn replace_placeholders(s: &str, placeholders: &[&str]) -> String {
    let prefix = PLACEHOLDER_PREFIX;
    let prefix_len = prefix.len();
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + prefix_len <= len
            && s.is_char_boundary(i + prefix_len)
            && &s[i..i + prefix_len] == prefix
        {
            // parse placeholder index
            let digit_start = i + prefix_len;
            let mut digit_end = digit_start;
            while digit_end < len && bytes[digit_end].is_ascii_digit() {
                digit_end += 1;
            }
            if digit_end > digit_start
                && let Ok(idx) = s[digit_start..digit_end].parse::<usize>()
                && let Some(original) = placeholders.get(idx)
            {
                result.push_str(original);

                // skip placeholder padding
                let mut pad_end = digit_end;
                while pad_end < len && bytes[pad_end] == 0x01 {
                    pad_end += 1;
                }
                i = pad_end;
                continue;
            }

            // copy invalid placeholder prefix
            let Some(ch) = s[i..].chars().next() else {
                break;
            };

            result.push(ch);
            i += ch.len_utf8();
        } else {
            let Some(ch) = s[i..].chars().next() else {
                break;
            };

            result.push(ch);
            i += ch.len_utf8();
        }
    }

    result
}
