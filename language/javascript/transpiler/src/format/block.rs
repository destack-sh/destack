use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::{Block, NodeId, Statement};

use crate::{FormatNode, JavaScriptFormatter};

pub(crate) fn format_block_of_statements<'ast>(
    f: &mut JavaScriptFormatter<'ast, '_>,
    statements: &Vec<NodeId<Statement>>,
) -> FormatResult<()> {
    f.join_with(hard_line_break()).entries(statements).finish()
}

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
                block_indent(&format_with(|f| format_block_of_statements(
                    f,
                    &self.statements
                ))),
                hard_line_break(),
                token("}")
            ]
        )?;
        Ok(())
    }
}
