use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{NodeId, Statement};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Statement> for Statement {
    fn format_node(
        &self,
        node_id: NodeId<Statement>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
