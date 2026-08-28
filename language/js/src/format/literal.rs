use crate::{Literal, StringLiteral, TemplateElement, TemplateLiteral, format_attributed};
use destack_core::StringId;
use destack_fir::format::{Format, FormatError, FormatResult, token};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::expression::format_expression_id_with_precedence;
use crate::{Context, Formatter, Precedence};

/// Format a scalar literal.
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &Literal,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    match scalar {
        Literal::Null => token("null").format(f)?,
        Literal::Undefined => token("undefined").format(f)?,
        Literal::Boolean(value) => token(if *value { "true" } else { "false" }).format(f)?,
        Literal::Bigint(value) => {
            let value_str = value.to_string();
            write!(f, [copied_text(&value_str), token("n")])?;
        }
        Literal::Number(value) => {
            let value_str = value.to_string();
            write!(f, [copied_text(&value_str)])?;
        }
        Literal::String(value) => {
            value.format(f)?;
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

/// Format a template literal.
pub(crate) fn format_template_literal<'ast>(
    template: &TemplateLiteral,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    if let Some(tag) = template.tag {
        format_expression_id_with_precedence(tag, Precedence::Postfix, f)?;
    }
    write!(f, [token("`"), template.head])?;

    for substitution in &template.substitutions {
        write!(
            f,
            [
                token("${"),
                substitution.expression,
                token("}"),
                substitution.tail
            ]
        )?;
    }

    write!(f, [token("`")])?;

    Ok(())
}

impl<'ast> Format<'ast, Context<'ast>> for Literal {
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

impl<'ast> Format<'ast, Context<'ast>> for StringLiteral {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        format_attributed(self.provenance, None, f, |f| {
            format_quoted_string_literal(self.value, f)
        })
    }
}

impl<'ast> Format<'ast, Context<'ast>> for TemplateElement {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        format_attributed(self.provenance, None, f, |f| self.raw.format(f))
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
