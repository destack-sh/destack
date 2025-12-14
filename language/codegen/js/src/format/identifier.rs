use crate::{Keyword, Name};
use destack_base::StringId;
use destack_fir::format::{Format, FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{CodegenJsFormatContext, CodegenJsFormatter};

impl<'ast> Format<CodegenJsFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<CodegenJsFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Name::Identifier(string) => string.format(f),
            Name::String(string) => write!(f, [token("\""), string, token("\"")]),
        }
    }
}
impl<'ast> Format<CodegenJsFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
