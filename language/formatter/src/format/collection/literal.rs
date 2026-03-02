use std::borrow::Cow;

use crate::format::expression::{is_expression_breakable, is_trivial_expression};
use crate::{DestackFormatContext, DestackFormatter};

use destack_ast::{
    Argument, Expression, FloatType, IfKind, IntType, LiteralType, LocalNodeId, Path,
    ScalarLiteral, TemplateLiteral, TypeLiteral,
};
use destack_base::StringId;
use destack_fir::format::{Format, FormatResult, text, token};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::QuoteStyle;

// template interpolation complexity thresholds
const TEMPLATE_COMPLEX_ARGUMENT_COUNT_THRESHOLD: usize = 2;
const TEMPLATE_COMPLEX_OBJECT_PROPERTY_THRESHOLD: usize = 2;

/// Format a path with dot separated segments.
impl<'ast> Format<DestackFormatContext<'ast>> for Path {
    /// Write every segment with `.` separators.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let mut segments = self.segments.iter().copied();
        let Some(first_segment) = segments.next() else {
            return Ok(());
        };

        write!(f, [first_segment])?;

        for segment in segments {
            write!(f, [token("."), segment])?;
        }

        Ok(())
    }
}

/// One token-level source snapshot for scalar literal formatting.
#[derive(Clone, Debug, Default)]
struct ScalarLiteralSource {
    source_lexeme: Option<String>,
    literal_type: Option<LiteralType>,
}

/// Collect source data for one scalar literal span.
fn scalar_literal_source_info(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> ScalarLiteralSource {
    let token = context.first_non_trivia_token_in_span(span);

    ScalarLiteralSource {
        source_lexeme: context.literal_lexeme_in_span(span).map(ToOwned::to_owned),
        literal_type: token.and_then(|token| token.token.literal),
    }
}

/// Escape string content for one quote-delimited literal.
fn escape_string_literal_content(content: &str, quote_char: char) -> String {
    let mut escaped = String::with_capacity(content.len());
    let mut characters = content.chars().peekable();

    while let Some(ch) = characters.next() {
        if ch == '\\' {
            let Some(next) = characters.next() else {
                escaped.push('\\');
                escaped.push('\\');
                break;
            };

            escaped.push('\\');
            escaped.push(next);
            continue;
        }

        match ch {
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            ch if ch == quote_char => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }

    escaped
}

/// Format a scalar literal.
/// (This is a separate function because it's not a node but we need the span for normalization.)
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &ScalarLiteral,
    span: Span,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let source_info = scalar_literal_source_info(f.context(), span);
    let source_lexeme = source_info.source_lexeme.unwrap_or_default();
    let literal_type = source_info.literal_type;
    let is_tree_text = literal_type == Some(LiteralType::TreeString);

    match scalar {
        ScalarLiteral::Boolean(value) => token(if *value { "true" } else { "false" }).format(f)?,
        ScalarLiteral::Integer(value) => {
            if source_lexeme.is_empty() {
                text(&value.to_string()).format(f)?;
            } else {
                let normalized = normalize_int(&source_lexeme, false);
                text(normalized.as_ref()).format(f)?;
            }
        }
        ScalarLiteral::Bigint(value) => {
            if source_lexeme.is_empty() {
                text(&format!("{value}n")).format(f)?;
            } else {
                let normalized = normalize_int(&source_lexeme, true);
                text(normalized.as_ref()).format(f)?;
            }
        }
        ScalarLiteral::Float(value) => {
            if source_lexeme.is_empty() {
                text(&value.to_string()).format(f)?;
            } else {
                let normalized = normalize_float(&source_lexeme);
                text(normalized.as_ref()).format(f)?;
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
                let escaped_content = escape_string_literal_content(content.as_str(), quote_char);
                write!(
                    f,
                    [
                        token(quote_str),
                        text(escaped_content.as_str()),
                        token(quote_str)
                    ]
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
            let escaped_content = escape_string_literal_content(content, quote_char);

            if is_tree_text {
                // jsx text content: normalize whitespace based on parsed tree text payload
                let has_newline = content.contains(['\n', '\r']);
                let has_non_whitespace = content
                    .chars()
                    .any(|character| !is_jsx_whitespace_char(character));
                if !has_non_whitespace {
                    if !has_newline {
                        write!(f, [text(" ")])?;
                    }
                } else if let Some(multiline_lines) = normalize_jsx_text_multiline_lines(content) {
                    let multiline = format_with(|f| {
                        for (line_index, line) in multiline_lines.iter().enumerate() {
                            if line_index > 0 {
                                write!(f, [hard_line_break()])?;
                            }
                            write!(f, [text(line)])?;
                        }

                        Ok(())
                    });
                    write!(f, [multiline])?;
                } else {
                    let normalized = normalize_jsx_text(content);
                    write!(f, [text(normalized.as_str())])?;
                }
            } else {
                let quote_str = if quote_char == '"' { "\"" } else { "'" };
                write!(
                    f,
                    [
                        token(quote_str),
                        text(escaped_content.as_str()),
                        token(quote_str)
                    ]
                )?;
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

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    // preserve multiline template interpolation intent from source
    let template_has_newline = f.context().has_newline(template_span);

    for (argument, segment) in arguments.iter().zip(string_segments) {
        let should_force_inline_ternary = template_argument_should_force_inline_ternary(
            f.context(),
            *argument,
            template_has_newline,
        );
        let should_expand =
            template_argument_should_expand(f.context(), *argument, template_has_newline);

        if should_force_inline_ternary {
            write!(
                f,
                [
                    group(&format_args![
                        token("${"),
                        *argument,
                        line_postfix_boundary(),
                        token("}")
                    ]),
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
                        line_postfix_boundary(),
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
                        line_postfix_boundary(),
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

/// Decide whether one template interpolation ternary should stay fully inline.
fn template_argument_should_force_inline_ternary(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    template_has_newline: bool,
) -> bool {
    let expression_id = template_argument_expression_id(context, argument_id);

    if context.node_has_newline(expression_id) || context.node_has_newline(argument_id) {
        return false;
    }

    if context.has_annotation(expression_id) || context.has_annotation(argument_id) {
        return false;
    }

    matches!(
        context.tree.get(expression_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) && !template_has_newline
}

/// Decide whether a template literal interpolation should break across lines.
fn template_argument_should_expand(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    template_has_newline: bool,
) -> bool {
    let expression_id = template_argument_expression_id(context, argument_id);
    let expression = context.tree.get(expression_id);
    if is_trivial_expression(context.tree, expression) {
        return false;
    }

    // preserve multiline template literals with conditional interpolation
    if template_has_newline
        && matches!(
            expression,
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        )
    {
        return true;
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

    let span = context.span(expression_id);
    let has_expression_newline = context.has_newline(span) || context.node_has_newline(argument_id);
    if !has_expression_newline {
        return false;
    }

    true
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

/// Normalize jsx text by collapsing whitespace to single spaces.
fn normalize_jsx_text(text: &str) -> String {
    let Some(mut normalized) = collapse_jsx_whitespace_to_single_spaces(text) else {
        return String::new();
    };

    if jsx_has_leading_inline_space(text) {
        normalized.insert(0, ' ');
    }

    if jsx_has_trailing_inline_space(text) {
        normalized.push(' ');
    }

    normalized
}

/// Normalize multiline jsx text into line-preserving segments.
fn normalize_jsx_text_multiline_lines(text: &str) -> Option<Vec<String>> {
    if !text.contains(['\n', '\r']) {
        return None;
    }

    let mut lines = text
        .lines()
        .filter_map(collapse_jsx_whitespace_to_single_spaces)
        .collect::<Vec<_>>();
    if lines.len() <= 1 {
        return None;
    }

    if let Some(first_line) = lines.first_mut()
        && jsx_has_leading_inline_space(text)
    {
        first_line.insert(0, ' ');
    }

    if let Some(last_line) = lines.last_mut()
        && jsx_has_trailing_inline_space(text)
    {
        last_line.push(' ');
    }

    Some(lines)
}

/// Return whether one jsx text node starts with inline boundary whitespace.
fn jsx_has_leading_inline_space(text: &str) -> bool {
    let leading_end = text
        .char_indices()
        .find(|(_, c)| !is_jsx_whitespace_char(*c))
        .map_or(text.len(), |(index, _)| index);

    let leading_whitespace = &text[..leading_end];
    !leading_whitespace.is_empty() && !leading_whitespace.contains(['\n', '\r'])
}

/// Return whether one jsx text node ends with inline boundary whitespace.
fn jsx_has_trailing_inline_space(text: &str) -> bool {
    let trailing_start = text
        .char_indices()
        .rev()
        .find(|(_, c)| !is_jsx_whitespace_char(*c))
        .map_or(0, |(index, c)| index + c.len_utf8());

    let trailing_whitespace = &text[trailing_start..];
    !trailing_whitespace.is_empty() && !trailing_whitespace.contains(['\n', '\r'])
}

/// Return whether one character is JSX whitespace.
#[inline]
fn is_jsx_whitespace_char(character: char) -> bool {
    matches!(character, ' ' | '\n' | '\r' | '\t')
}

/// Collapse JSX whitespace runs to single spaces and drop outer whitespace.
fn collapse_jsx_whitespace_to_single_spaces(text: &str) -> Option<String> {
    let mut collapsed = String::new();
    let mut saw_word = false;
    let mut has_pending_space = false;

    for character in text.chars() {
        if is_jsx_whitespace_char(character) {
            if saw_word {
                has_pending_space = true;
            }
            continue;
        }

        if has_pending_space && !collapsed.is_empty() {
            collapsed.push(' ');
        }

        collapsed.push(character);
        saw_word = true;
        has_pending_space = false;
    }

    (!collapsed.is_empty()).then_some(collapsed)
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

/// Normalize an integer literal lexeme without changing its base or bigint marker.
fn normalize_int(input: &str, _is_bigint: bool) -> Cow<'_, str> {
    let mut output = String::new();
    let mut last_index = 0;
    let mut is_hex = false;
    let mut characters = input.char_indices();

    if let Some((_, '0')) = characters.next()
        && let Some((index, character)) = characters.next()
    {
        is_hex = matches!(character, 'x' | 'X');
        if matches!(character, 'B' | 'O' | 'X' | 'b' | 'o' | 'x') {
            output.push('0');
            output.push(character.to_ascii_lowercase());
            last_index = index + character.len_utf8();
        }
    }

    if is_hex {
        for (index, character) in characters {
            if matches!(character, 'A'..='F') {
                output.push_str(&input[last_index..index]);
                output.push(character.to_ascii_lowercase());
                last_index = index + character.len_utf8();
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

/// Normalize one float literal lexeme without dropping exponent intent.
fn normalize_float(input: &str) -> Cow<'_, str> {
    let mut output = String::new();
    let mut last_index = 0;
    let mut characters = input.char_indices();
    let mut previous_character_is_dot = if let Some((index, '.')) = characters.next() {
        output.push('0');
        output.push('.');
        last_index = index + '.'.len_utf8();
        true
    } else {
        false
    };

    loop {
        match characters.next() {
            Some((index, character @ ('e' | 'E'))) => {
                if previous_character_is_dot {
                    output.push_str(&input[last_index..index]);
                    output.push('0');
                    last_index = index;
                }

                if character == 'E' {
                    output.push_str(&input[last_index..index]);
                    output.push('e');
                    last_index = index + 'E'.len_utf8();
                }

                if let Some((index, '+')) = characters.next() {
                    output.push_str(&input[last_index..index]);
                    last_index = index + '+'.len_utf8();
                }

                break;
            }
            Some((_index, character)) => {
                previous_character_is_dot = character == '.';
                continue;
            }
            None => {
                if previous_character_is_dot {
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
    use crate::{
        DestackFormatOptions, TestFormatter, assert_format, assert_format_roundtrip_with_file_type,
    };
    use destack_source::FileType;

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

    #[test]
    fn test_format_string_literal_escapes_embedded_target_quote() {
        assert_format!("'\"1\"'", r#""\"1\"""#, |p| p
            .eat_expression(Default::default()));
    }

    #[test]
    fn test_format_string_literal_does_not_double_escape_target_quote() {
        assert_format!(r#""\"1\"""#, r#""\"1\"""#, |p| p
            .eat_expression(Default::default()));
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

    /// Ternary interpolations in one-line templates should remain inline and idempotent.
    #[test]
    fn test_format_template_literal_ternary_interpolation_stays_inline_roundtrip() {
        assert_format_roundtrip_with_file_type(
            r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
            r#"`"${isSSR ? "------------------------------------------------------------------------------" : false}" TEST`"#,
            FileType::JavaScript,
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default(),
        );
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

    #[test]
    fn test_format_path_short() {
        assert_format!(
            "destack",
            "destack",
            |p| p.eat_path(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_multiple_segments() {
        assert_format!(
            "destack.geometry.math",
            "destack.geometry.math",
            |p| p.eat_path(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_path_with_overlong_line() {
        assert_format!(
            "destack.geometry.math.vector.point",
            "destack.geometry.math.vector.point",
            |p| p.eat_path(),
            DestackFormatOptions::default().with_line_width(20)
        );
    }
}
