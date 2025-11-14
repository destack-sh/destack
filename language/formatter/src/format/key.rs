use crate::{DystFormatContext, DystFormatter};
use dyst_ast::{Key, Keyword, Name};
use dyst_fir::format::text;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_source::StringId;

impl<'ast> Format<DystFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Name::Identifier(string) => string.format(f),
            Name::String(string) => write!(f, [token("\""), string, token("\"")]),
        }
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for Key {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
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

impl<'ast> Format<DystFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
