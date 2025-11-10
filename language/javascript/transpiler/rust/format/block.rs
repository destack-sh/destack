use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Block, NodeId};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        node_id: NodeId<Block>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
