use dyst_fir::format::{Format, FormatResult};
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::Key;

use crate::{JavaScriptFormatContext, JavaScriptFormatter};

impl<'ast> Format<JavaScriptFormatContext<'ast>> for Key {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Key::Name(name) => {
                write!(f, [name])?;
            }
            Key::Expression(expression) => {
                write!(f, [token("[")])?;
                write!(f, [expression])?;
                write!(f, [token("]")])?;
            }
            Key::NamedExpression { name, key } => {
                write!(f, [token("[")])?;
                write!(f, [name])?;
                write!(f, [token(":"), space()])?;
                write!(f, [key])?;
                write!(f, [token("]")])?;
            }
        }

        Ok(())
    }
}
