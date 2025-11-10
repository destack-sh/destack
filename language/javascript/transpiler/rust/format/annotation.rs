use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{Annotation, NodeId};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    fn format_node(
        &self,
        node_id: NodeId<Annotation>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
