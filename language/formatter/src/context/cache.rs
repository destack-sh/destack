use rustc_hash::FxHashMap;

use super::context::{DestackFormatContext, DestackFormatter};
use destack_fir::format::{Buffer, Format, FormatNode as FirNode, FormatNodes, FormatResult};
use destack_source::Span;

/// Formatted FIR nodes cached for one formatter pass.
#[derive(Debug, Default, Clone)]
pub struct FormatElementCache {
    /// Cached formatted elements keyed by source span.
    elements: FxHashMap<Span, FirNode>,
}

impl FormatElementCache {
    /// Return one cached formatted element for one source span.
    pub fn get(&self, span: &Span) -> Option<FirNode> {
        self.elements.get(span).cloned()
    }

    /// Cache one formatted element for one source span.
    pub fn insert(&mut self, span: Span, node: FirNode) {
        self.elements.insert(span, node);
    }
}

/// Preformatted content that can be inspected and emitted without formatting twice.
pub(crate) struct PreparedFormat {
    /// The interned formatted node.
    node: Option<FirNode>,
}

impl PreparedFormat {
    /// Format and intern one payload.
    pub(crate) fn new<'ast, T>(f: &mut DestackFormatter<'ast, '_>, content: T) -> FormatResult<Self>
    where
        T: Format<DestackFormatContext<'ast>>,
    {
        let node = f.intern(&content)?;

        Ok(Self { node })
    }

    /// Return whether the prepared content forces a line break.
    pub(crate) fn will_break(&self) -> bool {
        self.node.as_ref().is_some_and(FirNode::will_break)
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for PreparedFormat {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let Some(node) = self.node.clone() else {
            return Ok(());
        };

        f.write_node(node);
        Ok(())
    }
}
