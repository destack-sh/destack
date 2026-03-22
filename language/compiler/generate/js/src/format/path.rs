use crate::Path;
use destack_fir::format::FormatResult;

use destack_fir::prelude::*;
use destack_fir::write;

use crate::{CodegenJsFormatContext, CodegenJsFormatter};

impl<'ast> Format<CodegenJsFormatContext<'ast>> for Path {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        // a.b.c
        write!(
            f,
            [format_with(|f| f
                .join_with(token("."))
                .entries(&self.segments)
                .finish())]
        )
    }
}
