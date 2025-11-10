use crate::{DystFormatContext, DystFormatter, Keyword};
use dyst_ast::Name;
use dyst_fir::format::text;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_source::StringId;

impl<'ast> Format<DystFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(&string)])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Name::Identifier(string) => {
                string.format(f)?;
            }
            Name::String(string) => {
                write!(f, [token("\""), string, token("\"")])?;
            }
        };
        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
