use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, NodeId, Union};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Union> for Union {
    fn format_node(
        &self,
        _node_id: NodeId<Union>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
