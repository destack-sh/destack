use crate::{LanguageFormatter, Keyword, LanguageFormatContext};
use dyst_ast::Name;
use dyst_fir::format::text;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_source::StringId;

impl<'ast> Format<LanguageFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut LanguageFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().get_string(*self).to_string();
        write!(f, [text(&string)])
    }
}

impl<'ast> Format<LanguageFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut LanguageFormatter<'ast, '_>) -> FormatResult<()> {
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

impl<'ast> Format<LanguageFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut LanguageFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
