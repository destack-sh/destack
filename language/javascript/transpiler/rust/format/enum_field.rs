use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{EnumField, NodeId};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        node_id: NodeId<EnumField>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
