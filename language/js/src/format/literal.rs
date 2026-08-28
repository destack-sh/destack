use destack_core::StringId;
use destack_fir::format::{Format, FormatResult, token};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::expression::format_expression_id_with_precedence;
use crate::{
    Context, Formatter, Literal, Precedence, StringLiteral, TemplateElement, TemplateLiteral,
    format_attributed,
};

/// Format a scalar literal.
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &Literal,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    match scalar {
        Literal::Null => token("null").format(f)?,
        Literal::Undefined => write!(f, [token("void"), space(), token("0")])?,
        Literal::Boolean(value) => token(if *value { "true" } else { "false" }).format(f)?,
        Literal::Bigint(value) => {
            let value_str = value.to_string();
            write!(f, [copied_text(&value_str), token("n")])?;
        }
        Literal::Number(value) => {
            let value_str = if value.is_nan() {
                "0 / 0".to_string()
            } else if *value == f64::INFINITY {
                "1 / 0".to_string()
            } else if *value == f64::NEG_INFINITY {
                "-1 / 0".to_string()
            } else if *value == 0.0 && value.is_sign_negative() {
                "-0".to_string()
            } else {
                value.to_string()
            };
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
        let needs_parentheses = f
            .context()
            .tree
            .get(tag)
            .is_optional_chain(f.context().tree);
        if needs_parentheses {
            write!(f, [token("(")])?;
        }
        format_expression_id_with_precedence(tag, Precedence::Call, f)?;
        if needs_parentheses {
            write!(f, [token(")")])?;
        }
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
    let encoded = encode_js_string_literal(value, f);

    write!(f, [copied_text(&encoded)])?;

    Ok(())
}

/// Encode one string id as one quoted JavaScript string literal.
fn encode_js_string_literal<'ast>(value: StringId, f: &Formatter<'ast, '_>) -> String {
    let value = f.context().strings.get(value);
    let mut encoded = String::with_capacity(value.len() + 2);
    encoded.push('"');

    for character in value.chars() {
        match character {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\u{0008}' => encoded.push_str("\\b"),
            '\u{000c}' => encoded.push_str("\\f"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            '\u{2028}' => encoded.push_str("\\u2028"),
            '\u{2029}' => encoded.push_str("\\u2029"),
            '\u{0000}'..='\u{001f}' => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let value = character as usize;
                encoded.push_str("\\u00");
                encoded.push(HEX[(value >> 4) & 0x0f] as char);
                encoded.push(HEX[value & 0x0f] as char);
            }
            _ => encoded.push(character),
        }
    }

    encoded.push('"');

    encoded
}
