use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, Struct};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Struct> for Struct {
    fn format_node(
        &self,
        _node_id: NodeId<Struct>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
