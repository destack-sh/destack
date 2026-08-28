use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    Block, CatchClause, FormatNode, Formatter, Keyword, LocalNodeId, Statement, SwitchCase,
};

/// Format one statement sequence without its enclosing braces.
pub(crate) fn format_block_of_statements<'ast>(
    statements: &[LocalNodeId<Statement>],
    f: &mut Formatter<'ast, '_>,
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
        if self.statements.is_empty() {
            return write!(f, [token("{"), token("}")]);
        }

        // preserve multiline block layout
        write!(
            f,
            [
                token("{"),
                hard_line_break(),
                block_indent(&format_with(|f| format_block_of_statements(
                    &self.statements,
                    f
                ))),
                hard_line_break(),
                token("}")
            ]
        )?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for CatchClause {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [Keyword::Catch])?;

        if let Some(pattern) = self.pattern {
            write!(f, [space(), token("("), pattern, token(")")])?;
        }

        write!(f, [space(), self.body])
    }
}

impl<'ast> FormatNode<'ast> for SwitchCase {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        if let Some(value) = self.value {
            write!(f, [Keyword::Case, space(), value, token(":")])?;
        } else {
            write!(f, [Keyword::Default, token(":")])?;
        }

        if !self.body.is_empty() {
            write!(f, [hard_line_break()])?;
            write!(
                f,
                [block_indent(&format_with(|f| format_block_of_statements(
                    &self.body, f
                )))]
            )?;
        }

        Ok(())
    }
}
