use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::{Block, NodeId};

use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        _node_id: NodeId<Block>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Some(label) = &self.label {
            write!(f, [label, token(":"), space()])?;
        }
        write!(
            f,
            [
                token("{"),
                hard_line_break(),
                block_indent(&format_with(|f| f
                    .join_with(hard_line_break())
                    .entries(&self.statements)
                    .finish())),
                hard_line_break(),
                token("}")
            ]
        )?;
        Ok(())
    }
}
