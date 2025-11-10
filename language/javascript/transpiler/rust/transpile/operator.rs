use dyst_dir as dir;
use dyst_javascript_ast::UnaryOperator;

use crate::Transpiler;

impl<'a> Transpiler<'a> {
    /// Lower a DIR unary operator to a JavaScript unary operator.
    pub fn lower_unary_operator(&self, unary_operator: dir::UnaryOperator) -> UnaryOperator {
        todo!("lower_unary_operator {unary_operator:?}");
    }
}
