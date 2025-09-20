use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Try, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Try> for Try {
    fn format_node(
        &self,
        _node_id: NodeId<Try>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
