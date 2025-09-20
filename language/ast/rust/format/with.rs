use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, With};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, With> for With {
    fn format_node(
        &self,
        _node_id: NodeId<With>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
