use destack_fir::prelude::*;
use destack_fir::write;

use crate::{AssignOperator, BinaryOperator, Context, Formatter, UnaryOperator, UpdateOperator};

impl<'ast> Format<'ast, Context<'ast>> for UnaryOperator {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let token = token(self.as_str());
        write!(f, [token])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for UpdateOperator {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let token = token(self.as_str());
        write!(f, [token])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for BinaryOperator {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let token = token(self.as_str());
        write!(f, [token])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for AssignOperator {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let token = token(self.as_str());
        write!(f, [token])
    }
}
