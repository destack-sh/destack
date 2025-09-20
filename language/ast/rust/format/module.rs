use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Module, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Module> for Module {
    fn format_node(
        &self,
        _node_id: NodeId<Module>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
