use std::borrow::Cow;

use crate::template::{
    TemplateInterpolationIndentation, write_template_interpolation_with_indentation,
};
use crate::tree::is_jsx_whitespace_char;
use crate::{DestackFormatContext, DestackFormatter};

use destack_core::StringId;
use destack_dir::{
    Argument, Expression, FloatType, IntegerType, LocalNodeId, Path, ScalarLiteral,
    TemplateLiteral, TokenLiteral, TypeLiteral,
};
use destack_fir::format::{
    Buffer, Format, FormatNodes, FormatResult, RemoveSoftLinesBuffer, text, token,
};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::QuoteStyle;

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

/// Collect source data for one scalar literal span.
fn scalar_literal_source_info(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> (Option<String>, Option<TokenLiteral>) {
    let token = context.first_non_trivia_token_in_span(span);

    (
        context.literal_lexeme_in_span(span).map(ToOwned::to_owned),
        token.and_then(|token| token.token.literal),
    )
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

            let alternate_quote = if quote_char == '"' { '\'' } else { '"' };
            if next == alternate_quote {
                escaped.push(next);
                continue;
            }

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

/// Return the quote that minimizes escaped quote characters.
fn minimized_quote_char(content: &str, preferred_quote: char) -> char {
    let alternate_quote = if preferred_quote == '"' { '\'' } else { '"' };
    let preferred_count = content
        .chars()
        .filter(|character| *character == preferred_quote)
        .count();
    let alternate_count = content
        .chars()
        .filter(|character| *character == alternate_quote)
        .count();

    if preferred_count > alternate_count {
        alternate_quote
    } else {
        preferred_quote
    }
}

/// Format a scalar literal.
/// (This is a separate function because it's not a node but we need the span for normalization.)
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &ScalarLiteral,
    span: Span,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let (source_lexeme, literal_type) = scalar_literal_source_info(f.context(), span);
    let source_lexeme = source_lexeme.unwrap_or_default();
    let is_tree_text = literal_type == Some(TokenLiteral::TreeString);

    match scalar {
        ScalarLiteral::Null => token("null").format(f)?,
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
            let content = value.to_string();
            let escaped_content = escape_string_literal_content(content.as_str(), '\'');
            write!(f, [token("'"), text(escaped_content.as_str()), token("'")])?;
        }
        ScalarLiteral::String(string_id) => {
            let mut quote_style = f.context().options.quote_style;
            if quote_style == QuoteStyle::Semantic {
                quote_style = QuoteStyle::Double;
            }
            let content = f.context().strings.get(*string_id);
            let preferred_quote = quote_style.char_for(content);
            let quote_char = minimized_quote_char(content, preferred_quote);
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
    _template_span: Span,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), arguments.len().saturating_add(1));

    write!(f, [token("`")])?;

    let mut indentation = TemplateInterpolationIndentation::default();

    if let Some(first_segment) = strings.first() {
        write!(f, [*first_segment])?;
    }

    for (index, argument) in arguments.iter().enumerate() {
        let previous_segment = strings[index];
        let previous_segment_text = f.context().strings.get(previous_segment);
        indentation = TemplateInterpolationIndentation::after_last_newline(
            previous_segment_text,
            f.options().indent_width,
            indentation,
        );
        let after_newline = previous_segment_text.ends_with('\n');
        let next_segment = strings[index + 1];

        let format_argument = format_with(|f| write!(f, [*argument]));
        let interned_argument = f.intern(&format_argument)?;
        let layout = template_argument_layout(f.context(), *argument, &interned_argument);
        let format_inner = format_with(|f| {
            match layout {
                // single-line layout
                TemplateElementLayout::SingleLine => {
                    if let Some(interned_argument) = &interned_argument {
                        let mut buffer = RemoveSoftLinesBuffer::new(f);
                        buffer.write_node(interned_argument.clone());
                    }
                }
                // fit layout
                TemplateElementLayout::Fit => {
                    let should_indent =
                        template_argument_should_indent_fit_layout(f.context(), *argument);

                    match &interned_argument {
                        Some(interned_argument) if should_indent => {
                            write!(
                                f,
                                [soft_block_indent(&format_with(|f| {
                                    f.write_node(interned_argument.clone());
                                    Ok(())
                                }))]
                            )?;
                        }
                        Some(interned_argument) => {
                            f.write_node(interned_argument.clone());
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
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    interned_argument: &Option<destack_fir::format::FormatNode>,
) -> TemplateElementLayout {
    // preserve multiline interpolation expressions from source
    if template_argument_has_newline_in_range(context, argument_id) {
        return TemplateElementLayout::Fit;
    }

    // keep expressions that break in fit mode expandable
    if interned_argument
        .as_ref()
        .is_some_and(FormatNodes::will_break)
    {
        return TemplateElementLayout::Fit;
    }

    TemplateElementLayout::SingleLine
}

/// Return whether one interpolation argument spans a newline boundary in source.
fn template_argument_has_newline_in_range(
    context: &DestackFormatContext<'_>,
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
    context: &DestackFormatContext<'_>,
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
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::If { .. }
            | Expression::Match { .. }
            | Expression::Try { .. }
            | Expression::Comptime { .. }
            | Expression::Binary { .. }
            | Expression::Identifier { .. }
            | Expression::QualifiedReference { .. }
    )
}

/// Return the unwrapped expression id for a template interpolation argument.
fn template_argument_expression_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    let value = match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return None,
    };
    Some(unwrap_template_expression(context, value))
}

/// Unwrap a template interpolation argument into its underlying expression.
fn unwrap_template_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let _ = context;

    expression_id
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
    let Some(normalized) = collapse_jsx_whitespace_to_single_spaces(text) else {
        return String::new();
    };

    normalized
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
            TypeLiteral::Integer(int_type) => write!(f, [int_type]),
            TypeLiteral::Float(float_type) => write!(f, [float_type]),
            TypeLiteral::Symbol => write!(f, [token("symbol")]),
            TypeLiteral::UniqueSymbol => write!(f, [token("unique symbol")]),
            TypeLiteral::Intrinsic(_) => write!(f, [token("intrinsic")]),
        }?;

        Ok(())
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for IntegerType {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
        match self {
            IntegerType::Integer { is_signed } => {
                if *is_signed {
                    write!(f, [token("int")])
                } else {
                    write!(f, [token("uint")])
                }
            }
            IntegerType::Fixed { width, is_signed } => {
                if *is_signed {
                    write!(f, [token("int"), text(&width.to_string())])
                } else {
                    write!(f, [token("uint"), text(&width.to_string())])
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

impl<'ast> Format<DestackFormatContext<'ast>> for FloatType {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
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
