use crate::check::{Dump, DumpContext, Layout, LayoutQuery, LayoutShape, LayoutTerm};

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

impl Dump for Layout {
    /// Render one solved layout.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        let shape = match &self.shape {
            LayoutShape::None => "none",
            LayoutShape::Scalar => "scalar",
            LayoutShape::Struct(_) => "struct",
            LayoutShape::Tuple(_) => "tuple",
            LayoutShape::Slice(_) => "slice",
            LayoutShape::Array(_) => "array",
            LayoutShape::Variant(_) => "variant",
            LayoutShape::Object(_) => "object",
            LayoutShape::Dynamic => "dynamic",
            LayoutShape::Closure => "closure",
            LayoutShape::Newtype(_) => "newtype",
        };

        dump_record(
            "Layout",
            [
                ("shape", shape.to_string()),
                ("size", dump_optional_u32(self.size)),
                ("alignment", dump_optional_u32(self.alignment)),
            ],
        )
    }
}

/// Render one optional integer layout property.
fn dump_optional_u32(value: Option<u32>) -> String {
    value.map_or_else(|| "unknown".to_string(), |value| value.to_string())
}
