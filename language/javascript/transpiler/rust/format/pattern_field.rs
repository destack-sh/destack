use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{NodeId, PatternField};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, PatternField> for PatternField {
    fn format_node(
        &self,
        node_id: NodeId<PatternField>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
