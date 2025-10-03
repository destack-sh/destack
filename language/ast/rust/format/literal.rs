use std::borrow::Cow;

use crate::{
    CompositeType, DystFormatContext, DystFormatter, FloatType, IntType, Keyword, ScalarLiteral,
    TypeLiteral,
};
use dyst_fir::format::{Format, FormatResult, text, token};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> Format<DystFormatContext<'ast>> for ScalarLiteral {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let span = f.context().tree.get_span(node_id);
        let span_str = f.context().source.get_span_str(span);
        match self {
            ScalarLiteral::Boolean(value) => {
                token(if *value { "true" } else { "false" }).format(f)?
            }
            ScalarLiteral::Integer(_) => {
                let normalized = normalize_integer(span_str);
                text(&normalized).format(f)?;
            }
            ScalarLiteral::Float(_) => {
                let normalized = normalize_floating_number(span_str);
                text(&normalized).format(f)?;
            }
            ScalarLiteral::Character(value) => {
                write!(
                    f,
                    [token("'"), text(value.to_string().as_str()), token("'")]
                )?;
            }
            ScalarLiteral::Byte(value) => {
                write!(f, [token("b'"), text(&value.to_string()), token("'")])?;
            }
            ScalarLiteral::String(_) => {
                write!(f, [text(span_str)])?;
            }
            ScalarLiteral::ByteString(_) => {
                write!(f, [text(span_str)])?;
            }
        }

        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for TypeLiteral {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            TypeLiteral::Never => write!(f, [token("!")]),
            TypeLiteral::Any => write!(f, [token("$")]),
            TypeLiteral::Infer => write!(f, [token("_")]),
            TypeLiteral::Undefined => write!(f, [token("undefined")]),
            TypeLiteral::Void => write!(f, [token("void")]),
            TypeLiteral::Null => write!(f, [token("null")]),
            TypeLiteral::Boolean => write!(f, [token("boolean")]),
            TypeLiteral::Character => write!(f, [token("char")]),
            TypeLiteral::String => write!(f, [token("string")]),
            TypeLiteral::Number => write!(f, [token("number")]),
            TypeLiteral::Int(int_type) => write!(f, [int_type]),
            TypeLiteral::Float(float_type) => write!(f, [float_type]),
            TypeLiteral::Composite(composite_type) => write!(f, [composite_type]),
            TypeLiteral::Self_ => write!(f, [token("Self")]),
        }?;

        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for IntType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        if self.is_signed {
            // signed
            if let Some(width) = self.width {
                write!(f, [token("int"), text(&width.to_string())])
            } else {
                write!(f, [token("int")])
            }
        } else {
            // unsigned
            if let Some(width) = self.width {
                write!(f, [token("uint"), text(&width.to_string())])
            } else {
                write!(f, [token("uint")])
            }
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for FloatType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        if let Some(width) = self.width {
            write!(f, [token("float"), text(&width.to_string())])
        } else {
            write!(f, [token("float")])
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for CompositeType {
    fn format(&self, f: &mut Formatter<'_, DystFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            CompositeType::Type => write!(f, [Keyword::Type]),
            CompositeType::Struct => write!(f, [Keyword::Struct]),
            CompositeType::Enum => write!(f, [Keyword::Enum]),
            CompositeType::Union => write!(f, [Keyword::Union]),
            CompositeType::Tuple => write!(f, [Keyword::Tuple]),
            CompositeType::Trait => write!(f, [Keyword::Trait]),
            CompositeType::Function => write!(f, [Keyword::Function]),
        }
    }
}

/// Normalize an integer string to canonical form.
///
/// Lowercases prefixes (0b, 0o, 0x) and uppercases hex digits.
fn normalize_integer(input: &str) -> Cow<'_, str> {
    // normalized string if input is not yet normalized
    // output must remain empty if input is already normalized
    let mut output = String::new();
    // tracks the last index of input that has been written to output
    // if last_index is 0 at the end, then the input is already normalized and can be returned as is
    let mut last_index = 0;
    let mut is_hex = false;
    let mut chars = input.char_indices();

    // check if the input starts with a 0 and is followed by a B, O, or X
    if let Some((_, '0')) = chars.next()
        && let Some((index, c)) = chars.next()
    {
        is_hex = matches!(c, 'x' | 'X');
        if matches!(c, 'B' | 'O' | 'X' | 'b' | 'o' | 'x') {
            output.push('0');
            output.push(c.to_ascii_lowercase());
            last_index = index + c.len_utf8();
        }
    }

    // skip the rest if input is not a hex integer because there are only digits
    if is_hex {
        for (index, c) in chars {
            // uppercase hex digits
            if matches!(c, 'a'..='f') {
                output.push_str(&input[last_index..index]);
                output.push(c.to_ascii_uppercase());
                last_index = index + c.len_utf8();
            }
        }
    }

    if last_index == 0 {
        Cow::Borrowed(input)
    } else {
        output.push_str(&input[last_index..]);
        Cow::Owned(output)
    }
}

/// Normalize a floating point number string to canonical form.
///
/// Adds leading/trailing zeros where needed, lowercases exponent, removes plus sign.
fn normalize_floating_number(input: &str) -> Cow<'_, str> {
    // normalized string if input is not yet normalized
    // output must remain empty if input is already normalized
    let mut output = String::new();
    // tracks the last index of input that has been written to output
    // if last_index is 0 at the end, then the input is already normalized and can be returned as is
    let mut last_index = 0;
    let mut chars = input.char_indices();
    let mut prev_char_is_dot = if let Some((index, '.')) = chars.next() {
        // add a leading 0 if input starts with .
        output.push('0');
        output.push('.');
        last_index = index + '.'.len_utf8();
        true
    } else {
        false
    };

    loop {
        match chars.next() {
            Some((index, c @ ('e' | 'E'))) => {
                // add 0 if the e immediately follows a . (e.g., 1.e1)
                if prev_char_is_dot {
                    output.push_str(&input[last_index..index]);
                    output.push('0');
                    last_index = index;
                }

                // lowercase exponent part
                if c == 'E' {
                    output.push_str(&input[last_index..index]);
                    output.push('e');
                    last_index = index + 'E'.len_utf8();
                }

                // remove + in exponent part
                if let Some((index, '+')) = chars.next() {
                    output.push_str(&input[last_index..index]);
                    last_index = index + '+'.len_utf8();
                }

                break;
            }
            Some((_index, c)) => {
                prev_char_is_dot = c == '.';
                continue;
            }
            None => {
                if prev_char_is_dot {
                    // add 0 if fraction part ends with .
                    output.push_str(&input[last_index..]);
                    output.push('0');
                    last_index = input.len();
                }

                break;
            }
        }
    }

    if last_index == 0 {
        Cow::Borrowed(input)
    } else {
        output.push_str(&input[last_index..]);
        Cow::Owned(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_boolean_literal() {
        assert_format!("true", "true", |p| p.eat_scalar_literal());
        assert_format!("false", "false", |p| p.eat_scalar_literal());
    }

    #[test]
    fn test_format_integer_literal() {
        assert_format!(
            "1",
            "1",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_long() {
        assert_format!(
            "1_000_000",
            "1_000_000",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_hex() {
        assert_format!(
            "0x1234",
            "0x1234",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_hex_long() {
        assert_format!(
            "0x1234_5678",
            "0x1234_5678",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_integer_literal_binary() {
        assert_format!(
            "0b1010",
            "0b1010",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_float_literal() {
        assert_format!(
            "1.0",
            "1.0",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_float_literal_long() {
        assert_format!(
            "1.0e38",
            "1.0e38",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_string_literal() {
        assert_format!(
            "\"Hello, world!\"",
            "\"Hello, world!\"",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_string_literal_multiline() {
        assert_format!(
            "\"Hello, world!\nHello, world!\"",
            "\"Hello, world!\nHello, world!\"",
            |p| p.eat_scalar_literal(),
            DystFormatOptions::default()
        );
    }
}
