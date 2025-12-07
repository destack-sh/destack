use crate::{ScalarLiteral, TemplateLiteral};
use destack_fir::format::{Format, FormatResult, token};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{CodegenJsFormatContext, CodegenJsFormatter};

/// Format a scalar literal.
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &ScalarLiteral,
    f: &mut CodegenJsFormatter<'ast, '_>,
) -> FormatResult<()> {
    match scalar {
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

/// Format a template literal.
pub(crate) fn format_template_literal<'ast>(
    template: &TemplateLiteral,
    _f: &mut CodegenJsFormatter<'ast, '_>,
) -> FormatResult<()> {
    todo!("format_template_literal: {template:?}");
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
