use dyst_language_fir::format::FormatResult;

use crate::{Block, DystFormatter, FormatNode, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        _node_id: NodeId<Block>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
