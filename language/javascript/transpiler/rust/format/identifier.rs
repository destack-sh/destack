use dyst_fir::format::{Format, FormatResult, text};
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::Keyword;
use dyst_source::StringId;

use crate::{JavaScriptFormatContext, JavaScriptFormatter};

impl<'ast> Format<JavaScriptFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(&string)])
    }
}

impl<'ast> Format<JavaScriptFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}
