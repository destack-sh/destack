use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Definition, NodeId};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Definition> for Definition {
    fn format_node(
        &self,
        node_id: NodeId<Definition>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
