use crate::{Block, LocalNodeId, Statement};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{CodegenJsFormatter, FormatNode};

pub(crate) fn format_block_of_statements<'ast>(
    f: &mut CodegenJsFormatter<'ast, '_>,
    statements: &Vec<LocalNodeId<Statement>>,
) -> FormatResult<()> {
    for (index, statement_id) in statements.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [*statement_id])?;

        let statement = f.context().tree.get(*statement_id);
        if statement.needs_semicolon() {
            write!(f, [token(";")])?;
        }
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Block>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
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
