use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{NodeId, Type};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Type> for Type {
    fn format_node(
        &self,
        node_id: NodeId<Type>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        assert!(f.context().include_types(), "type in non-type context: {node_id:?}");
        todo!("format_node{self:?}");
    }
}
