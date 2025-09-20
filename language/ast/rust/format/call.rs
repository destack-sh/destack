use dyst_language_fir::format::FormatResult;

use crate::{Call, DystFormatter, FormatNode, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Call> for Call {
    fn format_node(
        &self,
        _node_id: NodeId<Call>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
