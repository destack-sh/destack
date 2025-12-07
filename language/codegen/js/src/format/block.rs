use crate::{Block, LocalNodeId, Statement};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatNode, CodegenJsFormatter};

pub(crate) fn format_block_of_statements<'ast>(
    f: &mut CodegenJsFormatter<'ast, '_>,
    statements: &Vec<LocalNodeId<Statement>>,
) -> FormatResult<()> {
    f.join_with(hard_line_break()).entries(statements).finish()
}

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Block>,
        f: &mut CodegenJsFormatter<'ast, '_>,
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
