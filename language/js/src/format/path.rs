use crate::Path;
use destack_fir::format::FormatResult;

use destack_fir::prelude::*;
use destack_fir::write;

use crate::{Context, Formatter};

impl<'ast> Format<'ast, Context<'ast>> for Path {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
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
