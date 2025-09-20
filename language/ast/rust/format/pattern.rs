use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, Pattern};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Pattern> for Pattern {
    fn format_node(
        &self,
        _node_id: NodeId<Pattern>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
