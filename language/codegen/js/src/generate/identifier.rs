use crate::{Keyword, Name};
use destack_fir::format::{Format, FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::StringId;

use crate::{JavaScriptFormatContext, JavaScriptFormatter};

impl<'ast> Format<JavaScriptFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<JavaScriptFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Name::Identifier(string) => string.format(f),
            Name::String(string) => write!(f, [token("\""), string, token("\"")]),
        }
    }
}
impl<'ast> Format<JavaScriptFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
