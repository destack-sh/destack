use dyst_dir::{Expression, NodeId};

use crate::{Compiler, EvaluateResult};

impl<'a> Compiler<'a> {
    /// Evaluate an Expression.
    pub fn evaluate_expression(&mut self, expression_id: NodeId<Expression>) -> EvaluateResult<()> {
        let _expression = self.session.tree.get(expression_id);
        // todo!("evaluate_expression({expression:?})")
        Ok(())
    }
}
