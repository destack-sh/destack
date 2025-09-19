use crate::{DystFormatContext, DystFormatter, Keyword};
use dyst_language_fir::format::text;
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;
use dyst_language_source::StringId;

impl<'ast> Format<DystFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().get_string(*self).to_string();
        write!(f, [text(&string)])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}