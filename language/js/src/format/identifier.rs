use crate::{Keyword, Name};
use tspp_core::StringId;
use tspp_fir::format::{Format, FormatResult, text};
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{Context, Formatter};

impl<'ast> Format<'ast, Context<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Name::Identifier(string) => string.format(f),
            Name::String(string) => write!(f, [token("\""), string, token("\"")]),
        }
    }
}
impl<'ast> Format<'ast, Context<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
