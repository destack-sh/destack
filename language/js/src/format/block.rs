use crate::{Block, LocalNodeId, Statement};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{FormatNode, Formatter};

pub(crate) fn format_block_of_statements<'ast>(
    f: &mut Formatter<'ast, '_>,
    statements: &[LocalNodeId<Statement>],
) -> FormatResult<()> {
    let mut printed_any = false;

    // emit each statement with the pretty block separator
    for statement_id in statements {
        if printed_any {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [statement_id])?;
        printed_any = true;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast> for Block {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
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
