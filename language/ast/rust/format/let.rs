use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Let, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Let> for Let {
    fn format_node(
        &self,
        _node_id: NodeId<Let>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
