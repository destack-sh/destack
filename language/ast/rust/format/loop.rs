use dyst_fir::format::FormatResult;

use crate::{DystFormatter, For, FormatNode, Keyword, Loop, NodeId, Runtime, While};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, While> for While {
    fn format_node(
        &self,
        node_id: NodeId<While>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        if let Some(runtime) = self.runtime
            && runtime == Runtime::Static
        {
            write!(f, [token("@")])?;
        }
        write!(
            f,
            [Keyword::While, space(), self.condition, space(), self.body]
        )?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, For> for For {
    fn format_node(
        &self,
        node_id: NodeId<For>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        if let Some(runtime) = self.runtime
            && runtime == Runtime::Static
        {
            write!(f, [token("@")])?;
        }
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
        )?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Loop> for Loop {
    fn format_node(
        &self,
        node_id: NodeId<Loop>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        if let Some(runtime) = self.runtime
            && runtime == Runtime::Static
        {
            write!(f, [token("@")])?;
        }
        write!(f, [Keyword::Loop, space(), self.body])?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}
