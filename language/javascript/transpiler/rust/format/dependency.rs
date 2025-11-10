use dyst_fir::format::FormatResult;
use dyst_javascript_ast::{DependencyItem, NodeId};
use dyst_fir::write;
use dyst_fir::prelude::*;

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        node_id: NodeId<DependencyItem>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!("format_node{self:?}");
    }
}
