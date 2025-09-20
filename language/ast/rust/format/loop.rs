use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, Loop};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Loop> for Loop {
    fn format_node(
        &self,
        _node_id: NodeId<Loop>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
