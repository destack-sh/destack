use crate::{Keyword, Name};
use destack_core::StringId;
use destack_fir::format::{Format, FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{JsFormatContext, JsFormatter};

impl<'ast> Format<JsFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut JsFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<JsFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut JsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Name::Identifier(string) => string.format(f),
            Name::String(string) => write!(f, [token("\""), string, token("\"")]),
        }
    }
}
impl<'ast> Format<JsFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut JsFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
