use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Key, Keyword, Name};
use destack_fir::format::text;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::StringId;

impl<'ast> Format<DestackFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Name::Identifier(string) => string.format(f),
            Name::String(string) => write!(f, [token("\""), string, token("\"")]),
            Name::Number(string) => string.format(f),
        }
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Key {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
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

impl<'ast> Format<DestackFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
