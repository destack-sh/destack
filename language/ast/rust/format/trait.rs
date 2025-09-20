use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, Trait};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Trait> for Trait {
    fn format_node(
        &self,
        _node_id: NodeId<Trait>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
