use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Function, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Function> for Function {
    fn format_node(
        &self,
        _node_id: NodeId<Function>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
