use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Argument, NodeId};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: NodeId<Argument>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
