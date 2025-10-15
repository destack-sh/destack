use dyst_dir::{Expression, NodeId};

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Evaluate an Expression.
    pub fn evaluate_expression(&mut self, expression_id: NodeId<Expression>) {
        let expression = self.tree.get(expression_id);
        todo!("evaluate_expression({expression:?})")
    }
}
