use crate::{ScalarLiteral, TemplateLiteral};
use destack_fir::format::{Format, FormatResult, token};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::{CodegenJsFormatContext, CodegenJsFormatter};

/// Format a scalar literal.
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &ScalarLiteral,
    f: &mut CodegenJsFormatter<'ast, '_>,
) -> FormatResult<()> {
    match scalar {
        ScalarLiteral::Null => token("null").format(f)?,
        ScalarLiteral::Undefined => token("undefined").format(f)?,
        ScalarLiteral::Boolean(value) => token(if *value { "true" } else { "false" }).format(f)?,
        ScalarLiteral::Bigint(value) => {
            let value_str = value.to_string();
            write!(f, [text(&value_str), token("n")])?;
        }
        ScalarLiteral::Number(value) => {
            let value_str = value.to_string();
            write!(f, [text(&value_str)])?;
        }
        ScalarLiteral::String(value) => {
            write!(f, [token("\""), *value, token("\"")])?;
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
    value: destack_core::StringId,
    source_span: Option<Span>,
    f: &mut CodegenJsFormatter<'ast, '_>,
) -> FormatResult<()> {
    // exact literal source span
    if let Some(source_span) = source_span {
        write!(
            f,
            [
                source_position(source_span.start),
                token("\""),
                value,
                token("\""),
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
    f: &mut CodegenJsFormatter<'ast, '_>,
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

impl<'ast> Format<CodegenJsFormatContext<'ast>> for ScalarLiteral {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        format_scalar_literal(self, f)
    }
}

impl<'ast> Format<CodegenJsFormatContext<'ast>> for TemplateLiteral {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        format_template_literal(self, f)
    }
}
