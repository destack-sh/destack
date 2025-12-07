use crate::Key;
use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{CodegenJsFormatContext, CodegenJsFormatter};

impl<'ast> Format<CodegenJsFormatContext<'ast>> for Key {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
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
