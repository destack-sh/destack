use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{NodeId, Pattern};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        node_id: NodeId<Pattern>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
