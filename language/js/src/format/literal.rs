use crate::{ScalarLiteral, TemplateLiteral};
use destack_core::StringId;
use destack_fir::format::{Format, FormatError, FormatResult, token};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::{Context, Formatter};

/// Format a scalar literal.
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &ScalarLiteral,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    match scalar {
        ScalarLiteral::Null => token("null").format(f)?,
        ScalarLiteral::Undefined => token("undefined").format(f)?,
        ScalarLiteral::Boolean(value) => token(if *value { "true" } else { "false" }).format(f)?,
        ScalarLiteral::Bigint(value) => {
            let value_str = value.to_string();
            write!(f, [copied_text(&value_str), token("n")])?;
        }
        ScalarLiteral::Number(value) => {
            let value_str = value.to_string();
            write!(f, [copied_text(&value_str)])?;
        }
        ScalarLiteral::String(value) => {
            format_quoted_string_literal(*value, f)?;
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

/// Format one string literal against one explicit source span.
pub(crate) fn format_string_literal_with_source_span<'ast>(
    value: StringId,
    source_span: Option<Span>,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    // exact literal source span
    if let Some(source_span) = source_span {
        let encoded = encode_js_string_literal(value, f)?;

        write!(
            f,
            [
                source_position(source_span.start),
                copied_text(&encoded),
                source_position(source_span.end)
            ]
        )?;

        return Ok(());
    }

    format_scalar_literal(&ScalarLiteral::String(value), f)
}

/// Format a template literal.
pub(crate) fn format_template_literal<'ast>(
    template: &TemplateLiteral,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    match template {
        TemplateLiteral::String { template } => {
            write!(f, [token("`"), *template, token("`")])?;
        }
        TemplateLiteral::TaggedString { tag, template } => {
            write!(f, [tag, token("`"), *template, token("`")])?;
        }
        TemplateLiteral::InterpolatedString {
            template,
            expressions,
        } => {
            write!(f, [token("`")])?;

            for (index, string) in template.iter().enumerate() {
                write!(f, [*string])?;

                if let Some(expression) = expressions.get(index) {
                    write!(f, [token("${"), expression, token("}")])?;
                }
            }

            write!(f, [token("`")])?;
        }
        TemplateLiteral::TaggedInterpolatedString {
            tag,
            template,
            expressions,
        } => {
            write!(f, [tag, token("`")])?;

            for (index, string) in template.iter().enumerate() {
                write!(f, [*string])?;

                if let Some(expression) = expressions.get(index) {
                    write!(f, [token("${"), expression, token("}")])?;
                }
            }

            write!(f, [token("`")])?;
        }
    }

    Ok(())
}

impl<'ast> Format<'ast, Context<'ast>> for ScalarLiteral {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        format_scalar_literal(self, f)
    }
}

impl<'ast> Format<'ast, Context<'ast>> for TemplateLiteral {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        format_template_literal(self, f)
    }
}

/// Format one quoted string literal with escaping.
fn format_quoted_string_literal<'ast>(
    value: StringId,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let encoded = encode_js_string_literal(value, f)?;

    write!(f, [copied_text(&encoded)])?;

    Ok(())
}

/// Encode one string id as one quoted JavaScript string literal.
fn encode_js_string_literal<'ast>(
    value: StringId,
    f: &Formatter<'ast, '_>,
) -> FormatResult<String> {
    let value = f.context().strings.get(value);

    serde_json::to_string(value).map_err(|_| FormatError::SyntaxError {
        message: "JavaScript string literal cannot be encoded",
    })
}
