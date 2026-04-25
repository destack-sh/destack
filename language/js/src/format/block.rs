use crate::{Block, LocalNodeId, Statement};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatNode, JsFormatter};

pub(crate) fn format_block_of_statements<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    statements: &[LocalNodeId<Statement>],
) -> FormatResult<()> {
    let mut printed_any = false;

    // emit each statement with the pretty block separator
    for statement_id in statements.iter().copied() {
        let statement = f.context().tree.get(statement_id);

        // type only statements
        if !f.context().include_types() && statement.is_type_only(f.context().tree) {
            continue;
        }

        if printed_any {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [statement_id])?;
        printed_any = true;

        // terminate statements that require semicolons
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
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // preserve multiline block layout
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
