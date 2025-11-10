use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Field, NodeId};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Field> for Field {
    fn format_node(
        &self,
        node_id: NodeId<Field>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
