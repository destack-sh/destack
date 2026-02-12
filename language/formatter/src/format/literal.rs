use std::borrow::Cow;

use crate::expression::{is_expression_breakable, is_trivial_expression};
use crate::{DestackFormatContext, DestackFormatter};

use destack_ast::{
    Argument, Expression, FloatType, IfKind, IntType, LocalNodeId, ScalarLiteral, TemplateLiteral,
    TypeLiteral,
};
use destack_base::StringId;
use destack_fir::format::{Format, FormatResult, text, token};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::QuoteStyle;

// template interpolation complexity thresholds
const TEMPLATE_COMPLEX_ARGUMENT_COUNT_THRESHOLD: usize = 2;
const TEMPLATE_INTERPOLATION_DELIMITER_WIDTH: usize = 4;
const TEMPLATE_COMPLEX_OBJECT_PROPERTY_THRESHOLD: usize = 2;

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
                text(&format!("{value}n")).format(f)?;
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
            if f.context().options.language_type.is_destack() {
                write!(
                    f,
                    [token("'"), text(value.to_string().as_str()), token("'")]
                )?;
            } else {
                let mut quote_style = f.context().options.quote_style;
                if quote_style == QuoteStyle::Semantic {
                    quote_style = QuoteStyle::Double;
                }
                let content = value.to_string();
                let quote_char = quote_style.char_for(content.as_str());
                let quote_str = if quote_char == '"' { "\"" } else { "'" };
                write!(
                    f,
                    [token(quote_str), text(content.as_str()), token(quote_str)]
                )?;
            }
        }
        ScalarLiteral::String(string_id) => {
            let mut quote_style = f.context().options.quote_style;
            if quote_style == QuoteStyle::Semantic
                && !f.context().options.language_type.is_destack()
            {
                quote_style = QuoteStyle::Double;
            }
            let content = f.context().strings.get(*string_id);
            let quote_char = quote_style.char_for(content);

            if span_str.is_empty() {
                // fallback: no source span available, format from string pool
                let quote_str = if quote_char == '"' { "\"" } else { "'" };
                write!(f, [token(quote_str), text(content), token(quote_str)])?;
            } else if span_str.starts_with('"') || span_str.starts_with('\'') {
                // quoted string: normalize to preferred quote style
                let source_quote = span_str.chars().next().unwrap_or_default();
                let has_matching_quote = span_str.len() >= 2 && span_str.ends_with(source_quote);
                if has_matching_quote {
                    let inner = &span_str[1..span_str.len() - 1];
                    let mut normalized = String::with_capacity(span_str.len());
                    normalized.push(quote_char);
                    normalized.push_str(inner);
                    normalized.push(quote_char);
                    write!(f, [text(normalized.as_str())])?;
                } else {
                    let quote_str = if quote_char == '"' { "\"" } else { "'" };
                    write!(f, [token(quote_str), text(content), token(quote_str)])?;
                }
            } else {
                // jsx text content (unquoted): normalize whitespace
                let has_newline = span_str.contains(['\n', '\r']);
                let has_non_whitespace = span_str.chars().any(|c| !c.is_whitespace());
                if !has_non_whitespace {
                    if !has_newline {
                        write!(f, [text(" ")])?;
                    }
                } else {
                    let normalized = normalize_jsx_text(span_str);
                    write!(f, [text(normalized.as_str())])?;
                }
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
    template_span: Span,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), arguments.len().saturating_add(1));
    let template_has_newline = f.context().has_newline(template_span);

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (argument, segment) in arguments.iter().zip(string_segments) {
        let should_force_inline =
            !template_has_newline && template_argument_should_force_inline(f.context(), *argument);
        let should_expand = template_argument_should_expand(f.context(), *argument);

        if should_force_inline {
            let expression_id = template_argument_expression_id(f.context(), *argument);
            let expression_span = f.context().get_span(expression_id);
            let raw_expression = f.context().get_span_str(expression_span).trim();
            write!(
                f,
                [
                    group(&format_args![token("${"), text(raw_expression), token("}")]),
                    *segment,
                ]
            )?;
        } else if should_expand {
            write!(
                f,
                [
                    group(&format_args![
                        token("${"),
                        indent(&format_args![hard_line_break(), argument]),
                        hard_line_break(),
                        token("}")
                    ])
                    .should_expand(true),
                    *segment,
                ]
            )?;
        } else {
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
    }

    write!(f, [token("`")])
}

/// Return the unwrapped expression id for a template interpolation argument.
fn template_argument_expression_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> LocalNodeId<Expression> {
    let value = match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };
    unwrap_template_expression(context, value)
}

/// Decide whether a template interpolation should stay fully inline.
fn template_argument_should_force_inline(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let expression_id = template_argument_expression_id(context, argument_id);
    let expression_is_inline_trivial =
        is_trivial_expression(context.tree, context.tree.get(expression_id))
            && !context.has_annotation(expression_id)
            && !context.has_annotation(argument_id)
            && !context.node_has_newline(expression_id)
            && !context.node_has_newline(argument_id);
    if expression_is_inline_trivial {
        return true;
    }

    matches!(
        context.tree.get(expression_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) && !context.node_has_newline(expression_id)
        && !context.node_has_newline(argument_id)
}

/// Decide whether a template literal interpolation should break across lines.
fn template_argument_should_expand(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let expression_id = template_argument_expression_id(context, argument_id);
    let expression = context.tree.get(expression_id);
    if is_trivial_expression(context.tree, expression) {
        return false;
    }

    // prefer inline conditional interpolations unless source already spans lines
    if matches!(
        expression,
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) && !context.node_has_newline(expression_id)
        && !context.node_has_newline(argument_id)
    {
        return false;
    }

    if template_expression_is_complex(context, expression_id) {
        return true;
    }

    if !is_expression_breakable(context.tree, expression) {
        return false;
    }

    match expression {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => {
            if dynamic_arguments.len() > TEMPLATE_COMPLEX_ARGUMENT_COUNT_THRESHOLD {
                return true;
            }
        }
        _ => {}
    }

    let span = context.get_span(expression_id);
    let has_expression_newline = context.has_newline(span) || context.node_has_newline(argument_id);
    if !has_expression_newline {
        return false;
    }

    let span_str = context.get_span_str(span);
    let expression_len = span_str.chars().count();
    let line_width = usize::from(context.options.line_width);

    expression_len.saturating_add(TEMPLATE_INTERPOLATION_DELIMITER_WIDTH) > line_width
}

/// Check whether a template literal interpolation is complex enough to force expansion.
fn template_expression_is_complex(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);
    match expression {
        Expression::TypeBinary { .. }
        | Expression::TypeConditional { .. }
        | Expression::TypeMapped { .. }
        | Expression::TypeTemplateLiteral { .. } => true,
        Expression::Maybe { .. } | Expression::Must { .. } => true,
        Expression::TreeExpression { .. } => true,
        Expression::ObjectExpression { properties, .. } => {
            properties.len() > TEMPLATE_COMPLEX_OBJECT_PROPERTY_THRESHOLD
        }
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => {
            dynamic_arguments.len() > TEMPLATE_COMPLEX_ARGUMENT_COUNT_THRESHOLD
                || dynamic_arguments.iter().any(|argument_id| {
                    let argument = context.tree.get(*argument_id);
                    !is_trivial_expression(
                        context.tree,
                        argument_value_expression(context, argument),
                    )
                })
        }
        Expression::Index { left, index, .. } => {
            index.is_some_and(|index_id| {
                let index_expression = context.tree.get(index_id);
                !is_trivial_expression(context.tree, index_expression)
            }) || template_expression_is_complex(context, *left)
        }
        Expression::Member { left, .. } => template_expression_is_complex(context, *left),
        Expression::Statement(inner_id) => template_expression_is_complex(context, *inner_id),
        Expression::Parenthesized { expression } => {
            template_expression_is_complex(context, *expression)
        }
        _ => false,
    }
}

/// Extract the expression value from an argument node.
fn argument_value_expression<'ast>(
    context: &DestackFormatContext<'ast>,
    argument: &Argument,
) -> &'ast Expression {
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };
    context.tree.get(value_id)
}

/// Unwrap a template interpolation argument into its underlying expression.
fn unwrap_template_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current = expression_id;

    loop {
        match context.tree.get(current) {
            Expression::Statement(inner_id) => {
                current = *inner_id;
            }
            Expression::Parenthesized { expression } => {
                current = *expression;
            }
            _ => return current,
        }
    }
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
            format_interpolated_template_literal(strings, arguments, _span, f)?;
        }
    }

    Ok(())
}

/// Normalize jsx text by collapsing whitespace to single spaces and preserving edges.
fn normalize_jsx_text(text: &str) -> String {
    let mut parts = text.split_whitespace();
    let Some(first) = parts.next() else {
        return String::new();
    };

    let mut normalized = String::from(first);
    for part in parts {
        normalized.push(' ');
        normalized.push_str(part);
    }

    let (has_leading_space, has_trailing_space) = jsx_boundary_spaces(text);
    if has_leading_space {
        normalized.insert(0, ' ');
    }
    if has_trailing_space {
        normalized.push(' ');
    }

    normalized
}

/// Check for inline boundary spaces in jsx text.
fn jsx_boundary_spaces(text: &str) -> (bool, bool) {
    let leading_end = text
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(text.len(), |(index, _)| index);
    let trailing_start = text
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(0, |(index, c)| index + c.len_utf8());

    let leading_whitespace = &text[..leading_end];
    let trailing_whitespace = &text[trailing_start..];

    let has_leading_space =
        !leading_whitespace.is_empty() && !leading_whitespace.contains(['\n', '\r']);
    let has_trailing_space =
        !trailing_whitespace.is_empty() && !trailing_whitespace.contains(['\n', '\r']);

    (has_leading_space, has_trailing_space)
}

impl<'ast> Format<DestackFormatContext<'ast>> for TypeLiteral {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            TypeLiteral::Never => write!(f, [token("never")]),
            TypeLiteral::Any => write!(f, [token("any")]),
            TypeLiteral::Infer => write!(f, [token("_")]),
            TypeLiteral::Undefined => write!(f, [token("undefined")]),
            TypeLiteral::Unknown => write!(f, [token("unknown")]),
            TypeLiteral::Object => write!(f, [token("object")]),
            TypeLiteral::Void => write!(f, [token("void")]),
            TypeLiteral::Null => write!(f, [token("null")]),
            TypeLiteral::Boolean => write!(f, [token("boolean")]),
            TypeLiteral::Character => write!(f, [token("char")]),
            TypeLiteral::String => write!(f, [token("string")]),
            TypeLiteral::Bigint => write!(f, [token("bigint")]),
            TypeLiteral::Number => write!(f, [token("number")]),
            TypeLiteral::Int(int_type) => write!(f, [int_type]),
            TypeLiteral::Float(float_type) => write!(f, [float_type]),
            TypeLiteral::Symbol => write!(f, [token("symbol")]),
            TypeLiteral::UniqueSymbol => write!(f, [token("unique symbol")]),
            TypeLiteral::Intrinsic(_) => write!(f, [token("intrinsic")]),
        }?;

        Ok(())
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for IntType {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            IntType::Pointer { is_signed } => {
                if *is_signed {
                    write!(f, [token("isize")])
                } else {
                    write!(f, [token("usize")])
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

    /// Multi-char strings use double quotes in semantic mode.
    #[test]
    fn test_format_string_literal_multi_char() {
        assert_format!("'hello'", "\"hello\"", |p| p
            .eat_expression(Default::default()));
    }

    /// Single-char strings use single quotes in semantic mode.
    #[test]
    fn test_format_string_literal_single_char() {
        assert_format!("\"a\"", "'a'", |p| p.eat_expression(Default::default()));
    }

    /// Empty strings use double quotes in semantic mode.
    #[test]
    fn test_format_string_literal_empty() {
        assert_format!("''", "\"\"", |p| p.eat_expression(Default::default()));
    }

    /// Formats a template literal string with no interpolation.
    #[test]
    fn test_format_template_literal_plain() {
        let source = "`hello`";
        assert_format!(source, source, |p| p.eat_expression(Default::default()));
    }

    /// Formats a template literal string with one interpolation.
    #[test]
    fn test_format_template_literal_one_interpolation() {
        let source = "tagged`hello ${name}`";
        assert_format!(source, source, |p| p.eat_expression(Default::default()));
    }

    /// Formats a template literal string where the entire content is interpolation.
    #[test]
    fn test_format_template_literal_all_interpolation() {
        let source = "sql`${stmt}`";
        assert_format!(source, source, |p| p.eat_expression(Default::default()));
    }

    /// Formats a template literal string with multiple adjacent interpolations.
    #[test]
    fn test_format_template_literal_adjacent_interpolations() {
        let source = "`${start}${middle}${end}`";
        assert_format!(source, source, |p| p.eat_expression(Default::default()));
    }

    /// Formats a template literal with a complex SQL query and interpolation.
    #[test]
    fn test_format_template_literal_complex_sql() {
        let source = r#"sql.stmt`SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`"#;
        assert_format!(source, source, |p| p.eat_expression(Default::default()));
    }

    /// Long strings are NOT broken even when they exceed line width (like Prettier).
    #[test]
    fn test_format_long_string_not_broken() {
        let source =
            r#""This is a very long string that exceeds the line width but should not be broken""#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_with_line_width(40)
        );
    }

    /// Long template literals are NOT broken even when they exceed line width.
    #[test]
    fn test_format_long_template_literal_not_broken() {
        let source = r#"`This is a very long template literal that exceeds the line width but should not be broken`"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_with_line_width(40)
        );
    }
}
