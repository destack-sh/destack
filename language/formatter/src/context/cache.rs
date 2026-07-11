use rustc_hash::FxHashMap;

use super::context::{DestackFormatContext, DestackFormatter};
use destack_fir::format::{Buffer, Format, FormatNode as FirNode, FormatNodes, FormatResult};
use destack_source::ByteRange;

/// FIR nodes cached for one formatter pass.
#[derive(Debug, Default)]
pub struct FormatNodeCache<'a> {
    /// Cached FIR nodes keyed by file-local range.
    nodes: FxHashMap<ByteRange, FirNode<'a>>,
}

impl<'a> FormatNodeCache<'a> {
    /// Return one cached FIR node for a file-local range.
    pub fn get(&self, range: ByteRange) -> Option<FirNode<'a>> {
        self.nodes.get(&range).cloned()
    }

    /// Cache one FIR node for a file-local range.
    pub fn insert(&mut self, range: ByteRange, node: FirNode<'a>) {
        self.nodes.insert(range, node);
    }
}

/// Preformatted content that can be inspected and emitted without formatting twice.
pub(crate) struct PreparedFormat<'a> {
    /// The formatted node.
    node: Option<FirNode<'a>>,
}

impl<'ast> PreparedFormat<'ast> {
    /// Capture one formatted payload.
    pub(crate) fn new<T>(f: &mut DestackFormatter<'ast, '_>, content: T) -> FormatResult<Self>
    where
        T: Format<'ast, DestackFormatContext<'ast>>,
    {
        let node = f.capture(&content)?;

        Ok(Self { node })
    }

    /// Return whether the prepared content forces a line break.
    pub(crate) fn will_break(&self) -> bool {
        self.node.as_ref().is_some_and(FirNode::will_break)
    }
}

impl<'ast> Format<'ast, DestackFormatContext<'ast>> for PreparedFormat<'ast> {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let Some(node) = self.node.clone() else {
            return Ok(());
        };

        f.write_node(node);
        Ok(())
    }
}
