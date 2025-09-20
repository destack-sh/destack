use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, Match, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Match> for Match {
    fn format_node(
        &self,
        _node_id: NodeId<Match>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        todo!()
    }
}
