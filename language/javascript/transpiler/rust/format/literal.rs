use dyst_fir::format::{Format, FormatResult, token};
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::ScalarLiteral;

use crate::JavaScriptFormatter;

/// Format a scalar literal.
pub(crate) fn format_scalar_literal<'ast>(
    scalar: &ScalarLiteral,
    f: &mut JavaScriptFormatter<'ast, '_>,
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
