use crate::format::expression::format_expression_id_with_precedence;
use crate::{Declarator, FormatNode, Formatter, Precedence};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast> for Declarator {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let Declarator { pattern, value } = self;

        // pattern
        write!(f, [pattern])?;

        // value
        if let Some(value) = value {
            write!(f, [space()])?;
            write!(f, [token("=")])?;
            write!(f, [space()])?;
            format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
        }

        Ok(())
    }
}
