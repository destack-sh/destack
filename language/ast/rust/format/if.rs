use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, If, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, If> for If {
    fn format_node(
        &self,
        _node_id: NodeId<If>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
