use std::borrow::Cow;

use crate::template::{
    TemplateInterpolationIndentation, write_template_interpolation_with_indentation,
};
use crate::tree::is_tree_whitespace_char;
use crate::{TsppFormatContext, TsppFormatter};

use tspp_dir::{
    Argument, Expression, FloatType, IntegerType, Literal, LocalNodeId, Path, TemplateChunk,
    TemplateLiteral, TokenLiteral, TypeLiteral,
};
use tspp_fir::format::{Format, FormatLayout, FormatResult, token};
use tspp_fir::prelude::*;
use tspp_fir::{format_args, write};
use tspp_source::Span;

/// Format a path with dot separated segments.
impl<'ast> Format<'ast, TsppFormatContext<'ast>> for Path {
    /// Write every segment with `.` separators.
    fn format(&self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
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

/// Collect source data for one scalar literal span.
fn scalar_literal_source_info(
    context: &TsppFormatContext<'_>,
    span: Span,
) -> (Option<String>, Option<TokenLiteral>) {
    let token = context.first_token_in_span(span);

    (
        context.literal_lexeme_in_span(span).map(ToOwned::to_owned),
        token.and_then(|token| token.token.literal()),
    )
}

/// Re-escape written string content for one quote-delimited literal, normalizing its quotes.
fn escape_written_string_content(content: &str, quote_char: char) -> String {
    let mut escaped = String::with_capacity(content.len());
    let mut characters = content.chars();
    while let Some(ch) = characters.next() {
        // keep a written escape sequence, dropping the escape of the other quote
        if ch == '\\' {
            let alternate_quote = if quote_char == '"' { '\'' } else { '"' };
            match characters.next() {
                Some(next) if next == alternate_quote => escaped.push(next),
                Some(next) => {
                    escaped.push('\\');
                    escaped.push(next);
                }
                None => escaped.push_str("\\\\"),
            }

            continue;
        }
        push_escaped_char(&mut escaped, ch, quote_char);
    }

    escaped
}

/// Escape decoded string content for one quote-delimited literal.
fn escape_string_content(content: &str, quote_char: char) -> String {
    let mut escaped = String::with_capacity(content.len());
    for ch in content.chars() {
        push_escaped_char(&mut escaped, ch, quote_char);
    }

    escaped
}

/// Push one decoded character as it is written inside a quote-delimited literal.
fn push_escaped_char(escaped: &mut String, ch: char, quote_char: char) {
    match ch {
        '\\' => escaped.push_str("\\\\"),
        '\n' => escaped.push_str("\\n"),
        '\r' => escaped.push_str("\\r"),
        '\t' => escaped.push_str("\\t"),
        '\u{08}' => escaped.push_str("\\b"),
        '\u{0C}' => escaped.push_str("\\f"),
        ch if ch == quote_char => {
            escaped.push('\\');
            escaped.push(ch);
        }
        ch => escaped.push(ch),
    }
}

/// Format a scalar literal.
/// (This is a separate function because it's not a node but we need the span for normalization.)
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &Literal,
    span: Span,
    f: &mut TsppFormatter<'ast, '_>,
) -> FormatResult<()> {
    let (source_lexeme, literal_type) = scalar_literal_source_info(f.context(), span);
    let source_lexeme = source_lexeme.unwrap_or_default();
    let is_tree_text = literal_type == Some(TokenLiteral::TreeString);

    match scalar {
        Literal::Null => token("null").format(f)?,
        Literal::Undefined => token("undefined").format(f)?,
        Literal::Boolean(value) => token(if *value { "true" } else { "false" }).format(f)?,
        Literal::Integer(value) => {
            if source_lexeme.is_empty() {
                copied_text(&value.to_string()).format(f)?;
            } else {
                let normalized = normalize_int(&source_lexeme, false);
                copied_text(normalized.as_ref()).format(f)?;
            }
        }
        Literal::Bigint(value) => {
            if source_lexeme.is_empty() {
                copied_text(&format!("{value}n")).format(f)?;
            } else {
                let normalized = normalize_int(&source_lexeme, true);
                copied_text(normalized.as_ref()).format(f)?;
            }
        }
        Literal::Float(value) => {
            if source_lexeme.is_empty() {
                copied_text(&value.to_string()).format(f)?;
            } else {
                let normalized = normalize_float(&source_lexeme);
                copied_text(normalized.as_ref()).format(f)?;
            }
        }
        Literal::Character(value) => {
            let content = value.to_string();
            let escaped_content = escape_string_content(content.as_str(), '\'');
            write!(
                f,
                [
                    token("'"),
                    copied_text(escaped_content.as_str()),
                    token("'")
                ]
            )?;
        }
        Literal::String(string_id) => {
            let content = f.context().strings.get(*string_id);
            let quote_char = '"';

            // re-escape the written content where the source carries it, else the decoded content
            let written = source_lexeme
                .strip_prefix(['"', '\''])
                .and_then(|inner| inner.strip_suffix(['"', '\'']));
            let escaped_content = match written {
                Some(written) => escape_written_string_content(written, quote_char),
                None => escape_string_content(content, quote_char),
            };

            if is_tree_text {
                // tree text content: normalize whitespace based on parsed tree text payload
                let has_newline = content.contains(['\n', '\r']);
                let has_non_whitespace = content
                    .chars()
                    .any(|character| !is_tree_whitespace_char(character));
                if !has_non_whitespace {
                    if !has_newline {
                        write!(f, [space()])?;
                    }
                } else {
                    let normalized = normalize_tree_text(content);
                    write!(f, [copied_text(normalized.as_str())])?;
                }
            } else {
                let quote_str = if quote_char == '"' { "\"" } else { "'" };
                write!(
                    f,
                    [
                        token(quote_str),
                        copied_text(escaped_content.as_str()),
                        token(quote_str)
                    ]
                )?;
            }
        }
        Literal::RegexString { content, flags } => {
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
    chunks: &[TemplateChunk],
    arguments: &[LocalNodeId<Argument>],
    _template_span: Span,
    f: &mut TsppFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(chunks.len(), arguments.len().saturating_add(1));

    write!(f, [token("`")])?;

    let mut indentation = TemplateInterpolationIndentation::default();

    if let Some(first_chunk) = chunks.first() {
        write!(f, [first_chunk.raw])?;
    }

    for (index, argument) in arguments.iter().enumerate() {
        let previous_segment = chunks[index].raw;
        let previous_segment_text = f.context().strings.get(previous_segment);
        indentation = TemplateInterpolationIndentation::after_last_newline(
            previous_segment_text,
            f.options().indent_width,
            indentation,
        );
        let after_newline = previous_segment_text.ends_with('\n');
        let next_segment = chunks[index + 1].raw;

        let format_argument = format_with(|f| write!(f, [*argument]));
        let argument_element = f.capture(&format_argument)?;
        let layout = template_argument_layout(f.context(), *argument, argument_element.as_ref());
        let format_inner = format_with(|f| {
            match layout {
                // single-line layout
                TemplateElementLayout::SingleLine => {
                    if let Some(argument_element) = &argument_element
                        && let Some(argument_element) =
                            (*argument_element).remove_soft_lines(f.allocator())
                    {
                        f.write_element(argument_element);
                    }
                }
                // fit layout
                TemplateElementLayout::Fit => {
                    let should_indent =
                        template_argument_should_indent_fit_layout(f.context(), *argument);

                    match &argument_element {
                        Some(argument_element) if should_indent => {
                            write!(
                                f,
                                [soft_block_indent(&format_with(|f| {
                                    f.write_element(*argument_element);
                                    Ok(())
                                }))]
                            )?;
                        }
                        Some(argument_element) => {
                            f.write_element(*argument_element);
                        }
                        None => {}
                    }
                }
            }

            Ok(())
        });
        let format_indented = format_with(|f| {
            if after_newline {
                write!(f, [dedent_to_root(&format_inner)])?;
            } else {
                write_template_interpolation_with_indentation(&format_inner, indentation, f)?;
            }

            Ok(())
        });

        write!(
            f,
            [
                group(&format_args![
                    token("${"),
                    format_indented,
                    line_suffix_boundary(),
                    token("}")
                ]),
                next_segment,
            ]
        )?;
    }

    write!(f, [token("`")])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TemplateElementLayout {
    SingleLine,
    Fit,
}

/// Return the layout for one template interpolation argument.
fn template_argument_layout(
    context: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    argument_element: Option<&tspp_fir::format::FormatElement<'_>>,
) -> TemplateElementLayout {
    // preserve multiline interpolation expressions from source
    if template_argument_has_newline_in_range(context, argument_id) {
        return TemplateElementLayout::Fit;
    }

    // keep expressions that break in fit mode expandable
    if argument_element.is_some_and(FormatLayout::will_break) {
        return TemplateElementLayout::Fit;
    }

    TemplateElementLayout::SingleLine
}

/// Return whether one interpolation argument spans a newline boundary in source.
fn template_argument_has_newline_in_range(
    context: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(expression_id) = template_argument_expression_id(context, argument_id) else {
        return false;
    };

    let expression_span = context.span(expression_id);
    let source = context.source_text();

    source.has_newline_before(expression_span.start)
        || source.has_newline_after(expression_span.end)
        || source.contains_newline(expression_span)
}

/// Return whether one fit-layout interpolation should indent its body.
fn template_argument_should_indent_fit_layout(
    context: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(expression_id) = template_argument_expression_id(context, argument_id) else {
        return false;
    };

    if context.has_annotation(argument_id) || context.has_annotation(expression_id) {
        return true;
    }

    matches!(
        context.tree.get(expression_id),
        Expression::Member { .. }
            | Expression::Index { .. }
            | Expression::If { .. }
            | Expression::Match { .. }
            | Expression::Switch { .. }
            | Expression::Try { .. }
            | Expression::Const { .. }
            | Expression::Binary { .. }
            | Expression::Identifier { .. }
    )
}

/// Return the unwrapped expression id for a template interpolation argument.
fn template_argument_expression_id(
    context: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    let value = match context.tree.get(argument_id) {
        Argument::Positional { value } | Argument::Spread { value } => *value,
        Argument::Elision | Argument::Error => return None,
    };
    Some(unwrap_template_expression(context, value))
}

/// Unwrap a template interpolation argument into its underlying expression.
fn unwrap_template_expression(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let _ = context;

    expression_id
}

/// Format a template literal.
pub(crate) fn format_template_literal<'ast>(
    template: &TemplateLiteral,
    _span: Span,
    f: &mut TsppFormatter<'ast, '_>,
) -> FormatResult<()> {
    match template {
        TemplateLiteral::String { chunk } => {
            write!(f, [token("`"), chunk.raw, token("`")])?;
        }
        TemplateLiteral::InterpolatedString { chunks, arguments } => {
            format_interpolated_template_literal(chunks, arguments, _span, f)?;
        }
    }

    Ok(())
}

/// Normalize tree text by collapsing whitespace to single spaces.
fn normalize_tree_text(text: &str) -> String {
    let Some(normalized) = collapse_tree_whitespace_to_single_spaces(text) else {
        return String::new();
    };

    normalized
}

/// Collapse tree whitespace runs to single spaces and drop outer whitespace.
fn collapse_tree_whitespace_to_single_spaces(text: &str) -> Option<String> {
    let mut collapsed = String::new();
    let mut saw_word = false;
    let mut has_pending_space = false;

    for character in text.chars() {
        if is_tree_whitespace_char(character) {
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

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for TypeLiteral {
    fn format(&self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            TypeLiteral::Never => write!(f, [token("never")]),
            TypeLiteral::Undefined => write!(f, [token("undefined")]),
            TypeLiteral::Unknown => write!(f, [token("unknown")]),
            TypeLiteral::Void => write!(f, [token("void")]),
            TypeLiteral::Null => write!(f, [token("null")]),
            TypeLiteral::Boolean => write!(f, [token("boolean")]),
            TypeLiteral::Character => write!(f, [token("char")]),
            TypeLiteral::String => write!(f, [token("string")]),
            TypeLiteral::Bigint => write!(f, [token("bigint")]),
            TypeLiteral::Number => write!(f, [token("number")]),
            TypeLiteral::Alias(alias) => write!(f, [token(alias.as_str())]),
            TypeLiteral::Integer(int_type) => write!(f, [int_type]),
            TypeLiteral::Float(float_type) => write!(f, [float_type]),
        }?;

        Ok(())
    }
}

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for IntegerType {
    fn format(&self, f: &mut Formatter<'_, 'ast, TsppFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            IntegerType::Fixed { width, is_signed } => {
                if *is_signed {
                    write!(f, [token("int"), copied_text(&width.to_string())])
                } else {
                    write!(f, [token("uint"), copied_text(&width.to_string())])
                }
            }
            IntegerType::Pointer { is_signed } => {
                if *is_signed {
                    write!(f, [token("isize")])
                } else {
                    write!(f, [token("usize")])
                }
            }
        }
    }
}

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for FloatType {
    fn format(&self, f: &mut Formatter<'_, 'ast, TsppFormatContext<'ast>>) -> FormatResult<()> {
        write!(f, [token(self.as_str())])
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
