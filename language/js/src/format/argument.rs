use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::expression::format_expression_id_with_precedence;
use crate::{Argument, ArrayElement, FormatNode, Formatter, Precedence};

impl<'ast> FormatNode<'ast> for Argument {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Positional { value } => {
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
            }
            Self::Spread { value } => {
                write!(f, [token("...")])?;
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
            }
        }

        Ok(())
    }
}

impl<'ast> FormatNode<'ast> for ArrayElement {
    fn format_node(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Expression { value } => {
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
            }
            Self::Spread { value } => {
                write!(f, [token("...")])?;
                format_expression_id_with_precedence(*value, Precedence::Assignment, f)?;
            }
            Self::Elision => {}
        }

        Ok(())
    }
}
