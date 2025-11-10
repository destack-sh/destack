use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{NodeId, Parameter};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        node_id: NodeId<Parameter>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
