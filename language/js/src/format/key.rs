use crate::Key;
use tspp_fir::format::{Format, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{Context, Formatter};

impl<'ast> Format<'ast, Context<'ast>> for Key {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Key::Name(name) => {
                write!(f, [name])?;
            }
            Key::Private(name) => {
                write!(f, [token("#"), *name])?;
            }
            Key::Expression(expression) => {
                write!(f, [token("[")])?;
                write!(f, [expression])?;
                write!(f, [token("]")])?;
            }
        }

        Ok(())
    }
}
