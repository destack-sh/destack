use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, For, FormatNode, Keyword, Loop, NodeId, While};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;

impl<'ast> FormatNode<'ast, While> for While {
    fn format_node(
        &self,
        _node_id: NodeId<While>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [Keyword::While, space(), self.condition, space(), self.body]
        )
    }
}

impl<'ast> FormatNode<'ast, For> for For {
    fn format_node(
        &self,
        _node_id: NodeId<For>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [
                Keyword::For,
                space(),
                self.pattern,
                space(),
                Keyword::In,
                space(),
                self.iterator,
                space(),
                self.body
            ]
        )
    }
}

impl<'ast> FormatNode<'ast, Loop> for Loop {
    fn format_node(
        &self,
        _node_id: NodeId<Loop>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Loop, space(), self.body])
    }
}
