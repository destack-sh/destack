use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, Enum, FormatNode, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Enum> for Enum {
    fn format_node(
        &self,
        _node_id: NodeId<Enum>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
