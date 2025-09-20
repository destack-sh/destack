use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Implement, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Implement> for Implement {
    fn format_node(
        &self,
        _node_id: NodeId<Implement>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
