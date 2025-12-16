use std::borrow::Cow;

use crate::{DestackFormatContext, DestackFormatter};

use destack_ast::{
    Argument, DeclarationType, FloatType, IntType, Keyword, LocalNodeId, ScalarLiteral,
    TemplateLiteral, TypeLiteral,
};
use destack_base::StringId;
use destack_fir::format::{Format, FormatResult, text, token};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

/// Format a scalar literal.
/// (This is a separate function because it's not a node but we need the span for normalization.)
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &ScalarLiteral,
    span: Span,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let span_str = f.context().file.get_span_str(span).unwrap_or_default();
    match scalar {
        ScalarLiteral::Boolean(value) => token(if *value { "true" } else { "false" }).format(f)?,
        ScalarLiteral::Integer(value) => {
            if span_str.is_empty() {
                // fallback: no source span available, format from value
                text(&value.to_string()).format(f)?;
            } else {
                let normalized_str = normalize_int(span_str, false);
                text(&normalized_str).format(f)?;
            }
        }
        ScalarLiteral::Bigint(value) => {
            if span_str.is_empty() {
                // fallback: no source span available, format from value
                text(&format!("{}n", value)).format(f)?;
            } else {
                let normalized_str = normalize_int(span_str, true);
                text(&normalized_str).format(f)?;
            }
        }
        ScalarLiteral::Float(value) => {
            if span_str.is_empty() {
                // fallback: no source span available, format from value
                text(&value.to_string()).format(f)?;
            } else {
                let normalized_str = normalize_float(span_str);
                text(&normalized_str).format(f)?;
            }
        }
        ScalarLiteral::Character(value) => {
            write!(
                f,
                [token("'"), text(value.to_string().as_str()), token("'")]
            )?;
        }
        ScalarLiteral::String(string_id) => {
            if span_str.is_empty() {
                // fallback: no source span available, format from string pool
                let string = f.context().strings.get(*string_id);
                write!(f, [token("\""), text(string), token("\"")])?;
            } else if span_str.len() >= 2 && span_str.starts_with('\'') && span_str.ends_with('\'')
            {
                // single-quoted string -> convert to double quotes
                let mut normalized = String::with_capacity(span_str.len());
                normalized.push('"');
                normalized.push_str(&span_str[1..span_str.len() - 1]);
                normalized.push('"');
                write!(f, [text(normalized.as_str())])?;
            } else if span_str.starts_with('"') || span_str.starts_with('\'') {
                // quoted string -> use as-is
                write!(f, [text(span_str)])?;
            } else {
                // JSX text content (unquoted) -> trim whitespace
                write!(f, [text(span_str.trim())])?;
            }
        }
        ScalarLiteral::RegexString { content, flags } => {
            if let Some(flags) = flags {
                write!(f, [token("/"), content, token("/"), flags])?;
            } else {
                write!(f, [token("/"), content, token("/")])?;
            }
        }
    }

    Ok(())
}

/// Format an interpolated template literal.
/// |strings| = |arguments| + 1
fn format_interpolated_template_literal<'ast>(
    strings: &[StringId],
    arguments: &[LocalNodeId<Argument>],
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), arguments.len().saturating_add(1));

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (argument, segment) in arguments.iter().zip(string_segments) {
        write!(
            f,
            [
                group(&format_args![
                    token("${"),
                    indent(&format_args![soft_line_break(), argument]),
                    soft_line_break(),
                    token("}")
                ]),
                *segment,
            ]
        )?;
    }

    write!(f, [token("`")])
}

/// Format a template literal.
pub(crate) fn format_template_literal<'ast>(
    template: &TemplateLiteral,
    _span: Span,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    match template {
        TemplateLiteral::String { string } => {
            write!(f, [token("`"), string, token("`")])?;
        }
        TemplateLiteral::InterpolatedString { strings, arguments } => {
            format_interpolated_template_literal(strings, arguments, f)?;
        }
    }

    Ok(())
}

impl<'ast> Format<DestackFormatContext<'ast>> for TypeLiteral {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            TypeLiteral::Never => write!(f, [token("never")]),
            TypeLiteral::Any => write!(f, [token("any")]),
            TypeLiteral::Infer => write!(f, [token("_")]),
            TypeLiteral::Undefined => write!(f, [token("undefined")]),
            TypeLiteral::Unknown => write!(f, [token("unknown")]),
            TypeLiteral::Void => write!(f, [token("void")]),
            TypeLiteral::Null => write!(f, [token("null")]),
            TypeLiteral::Boolean => write!(f, [token("boolean")]),
            TypeLiteral::Character => write!(f, [token("char")]),
            TypeLiteral::String => write!(f, [token("string")]),
            TypeLiteral::Bigint => write!(f, [token("bigint")]),
            TypeLiteral::Number => write!(f, [token("number")]),
            TypeLiteral::Int(int_type) => write!(f, [int_type]),
            TypeLiteral::Float(float_type) => write!(f, [float_type]),
            TypeLiteral::Composite(composite_type) => write!(f, [composite_type]),
            TypeLiteral::Symbol => write!(f, [token("symbol")]),
            TypeLiteral::UniqueSymbol => write!(f, [token("unique symbol")]),
        }?;

        Ok(())
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for IntType {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            IntType::Pointer { is_signed } => {
                if *is_signed {
                    write!(f, [token("intp")])
                } else {
                    write!(f, [token("uintp")])
                }
            }
            IntType::Arbitrary { width, is_signed } => {
                if *is_signed {
                    // int
                    if let Some(width) = *width {
                        write!(f, [token("int"), text(&width.to_string())])
                    } else {
                        write!(f, [token("int")])
                    }
                } else {
                    // uint
                    if let Some(width) = *width {
                        write!(f, [token("uint"), text(&width.to_string())])
                    } else {
                        write!(f, [token("uint")])
                    }
                }
            }
        }
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for FloatType {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        if let Some(width) = self.width {
            write!(f, [token("float"), text(&width.to_string())])
        } else {
            write!(f, [token("float")])
        }
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for DeclarationType {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            DeclarationType::Type => write!(f, [Keyword::Type]),
            DeclarationType::Namespace => write!(f, [Keyword::Namespace]),
            DeclarationType::Struct => write!(f, [Keyword::Struct]),
            DeclarationType::Class => write!(f, [Keyword::Class]),
            DeclarationType::Enum => write!(f, [Keyword::Enum]),
            DeclarationType::Union => write!(f, [Keyword::Union]),
            DeclarationType::Interface => write!(f, [Keyword::Interface]),
            DeclarationType::Extension => write!(f, [Keyword::Extension]),
            DeclarationType::Function => write!(f, [Keyword::Function]),
        }
    }
}

/// Normalize an integer string to canonical form.
///
/// Lowercases prefixes (0b, 0o, 0x) and uppercases hex digits.
fn normalize_int(input: &str, _is_bigint: bool) -> Cow<'_, str> {
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
fn normalize_float(input: &str) -> Cow<'_, str> {
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
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    /// Strings parsed with single quotes should be rewritten with double quotes.
    #[test]
    fn test_format_string_literal_single_quote_leniency() {
        assert_format!("'hello'", "\"hello\"", |p| p.eat_expression());
    }

    /// Formats a template literal string with no interpolation.
    #[test]
    fn test_format_template_literal_plain() {
        let source = "`hello`";
        assert_format!(source, source, |p| p.eat_expression());
    }

    /// Formats a template literal string with one interpolation.
    #[test]
    fn test_format_template_literal_one_interpolation() {
        let source = "tagged`hello ${name}`";
        assert_format!(source, source, |p| p.eat_expression());
    }

    /// Formats a template literal string where the entire content is interpolation.
    #[test]
    fn test_format_template_literal_all_interpolation() {
        let source = "sql`${stmt}`";
        assert_format!(source, source, |p| p.eat_expression());
    }

    /// Formats a template literal string with multiple adjacent interpolations.
    #[test]
    fn test_format_template_literal_adjacent_interpolations() {
        let source = "`${start}${middle}${end}`";
        assert_format!(source, source, |p| p.eat_expression());
    }

    /// Formats a template literal with a complex SQL query and interpolation.
    #[test]
    fn test_format_template_literal_complex_sql() {
        let source = r#"sql.stmt`SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`"#;
        assert_format!(source, source, |p| p.eat_expression());
    }
}
