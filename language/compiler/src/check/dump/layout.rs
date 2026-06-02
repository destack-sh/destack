use crate::check::{Dump, DumpContext, LayoutQuery, LayoutTerm};

use super::format::dump_record;

impl Dump for LayoutTerm {
    /// Render one layout query term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "LayoutTerm",
            [
                ("source", context.node_label(self.source)),
                ("target", self.target.dump(context)),
                ("query", self.query.dump(context)),
            ],
        )
    }
}

impl Dump for LayoutQuery {
    /// Render one layout query.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Size => "size".to_string(),
            Self::Alignment => "alignment".to_string(),
            Self::Stride => "stride".to_string(),
        }
    }
}
