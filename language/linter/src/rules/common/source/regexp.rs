use destack_dir::{self as dir};

use crate::rules::common::expression_path_segments;

/// Regex pattern info extracted from one source expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegexPatternInfo {
    /// The regex pattern string id.
    pub pattern_id: dir::StringId,
    /// Optional regex flags string id when statically known.
    pub flags_id: Option<dir::StringId>,
    /// Whether constructor flags are present but not statically known.
    pub has_unknown_flags: bool,
}

/// Return canonical global qualifier names for `RegExp` constructor lookups.
pub fn regexp_global_qualifier_names(strings: &dir::StringPool) -> [dir::StringId; 4] {
    [
        strings.intern("globalThis"),
        strings.intern("window"),
        strings.intern("self"),
        strings.intern("global"),
    ]
}

/// Resolve regex pattern info from a regex literal or `RegExp` constructor call.
pub fn regex_pattern_info(
    strings: &dir::StringPool,
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    regexp_name: dir::StringId,
    global_qualifier_names: &[dir::StringId],
) -> Option<RegexPatternInfo> {
    // normalize expression shape
    let expression_id = super::expression_unwrap_parenthesized_source_form(tree, expression_id);
    let expression = tree.get(expression_id);

    // support direct regex literals
    if let dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString { content, flags }) =
        expression
    {
        return Some(RegexPatternInfo {
            pattern_id: *content,
            flags_id: *flags,
            has_unknown_flags: false,
        });
    }

    // support `RegExp(...)` and `new RegExp(...)`
    let (callee_id, arguments) = match expression {
        dir::Expression::Call {
            left, arguments, ..
        }
        | dir::Expression::New {
            left, arguments, ..
        } => (*left, arguments.as_slice()),
        _ => return None,
    };

    // require global RegExp constructor identifier
    let path_segments = expression_path_segments(tree, callee_id)?;
    if !path_is_regexp_constructor(
        path_segments.as_slice(),
        regexp_name,
        global_qualifier_names,
    ) {
        return None;
    }

    // require first positional string pattern argument
    let first_argument_id = *arguments.first()?;
    let first_argument = tree.get(first_argument_id);
    let dir::Argument::Positional {
        value: pattern_value,
        ..
    } = first_argument
    else {
        return None;
    };
    let pattern_expression = tree.get(*pattern_value);
    let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(pattern_string_id)) =
        pattern_expression
    else {
        return None;
    };
    let pattern = {
        let pattern_text = strings.get(*pattern_string_id);
        decode_string_content(pattern_text)?
    };
    let pattern_id = strings.intern(pattern.as_ref());

    // resolve optional second positional flags argument when statically known
    let mut has_unknown_flags = false;
    let flags_id = arguments.get(1).and_then(|flags_argument_id| {
        let flags_argument = tree.get(*flags_argument_id);
        let dir::Argument::Positional {
            value: flags_value, ..
        } = flags_argument
        else {
            has_unknown_flags = true;
            return None;
        };
        let flags_expression = tree.get(*flags_value);
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(flags_string_id)) =
            flags_expression
        else {
            has_unknown_flags = true;
            return None;
        };
        let flags = {
            let flags_text = strings.get(*flags_string_id);
            decode_string_content(flags_text)?
        };
        let flags_id = strings.intern(&flags);

        Some(flags_id)
    });

    Some(RegexPatternInfo {
        pattern_id,
        flags_id,
        has_unknown_flags,
    })
}

/// Return true when one path resolves to a global `RegExp` constructor.
pub fn path_is_regexp_constructor(
    path_segments: &[dir::StringId],
    regexp_name: dir::StringId,
    global_qualifier_names: &[dir::StringId],
) -> bool {
    if path_segments == [regexp_name] {
        return true;
    }

    if path_segments.len() != 2 {
        return false;
    }

    path_segments[1] == regexp_name && global_qualifier_names.contains(&path_segments[0])
}

/// Decode one string-literal content into runtime string value semantics.
fn decode_string_content(raw_content: &str) -> Option<String> {
    let mut decoded = String::new();
    let mut chars = raw_content.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }

        let escaped = chars.next()?;
        let replacement = match escaped {
            '\'' => '\'',
            '"' => '"',
            '`' => '`',
            '\\' => '\\',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'b' => '\u{0008}',
            'f' => '\u{000C}',
            'v' => '\u{000B}',
            // keep `\0` as null and reject legacy octal tails
            '0' => {
                if chars.peek().is_some_and(|next| next.is_ascii_digit()) {
                    return None;
                }
                '\0'
            }
            // decode 2-digit hex escapes
            'x' => {
                let high = chars.next()?.to_digit(16)?;
                let low = chars.next()?.to_digit(16)?;
                char::from_u32((high << 4) + low)?
            }
            // decode fixed and braced unicode escapes
            'u' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    let mut code_point = 0u32;
                    let mut has_digit = false;
                    let mut closed = false;
                    for next_character in chars.by_ref() {
                        if next_character == '}' {
                            closed = true;
                            break;
                        }
                        let digit = next_character.to_digit(16)?;
                        code_point = code_point.checked_mul(16)?.checked_add(digit)?;
                        has_digit = true;
                    }
                    if !has_digit || !closed {
                        return None;
                    }
                    char::from_u32(code_point)?
                } else {
                    let mut code_point = 0u32;
                    for _ in 0..4 {
                        let digit = chars.next()?.to_digit(16)?;
                        code_point = (code_point << 4) + digit;
                    }
                    char::from_u32(code_point)?
                }
            }
            // decode line continuations into nothing
            '\n' => continue,
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                continue;
            }
            '\u{2028}' | '\u{2029}' => continue,
            // keep regular escaped characters as the escaped character itself
            _ => escaped,
        };

        decoded.push(replacement);
    }

    Some(decoded)
}
