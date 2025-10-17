use dyst_dir::{NodeId, Type};

use crate::{Compiler, EvaluateResult};

impl<'a> Compiler<'a> {
    /// Evaluate a Type.
    pub fn evaluate_type(&mut self, ty_id: NodeId<Type>) -> EvaluateResult<()> {
        let ty = self.tree.get(ty_id);
        let Type::UnevaluatedExpression(expression_id) = ty else {
            return Ok(());
        };

        let expression = self.tree.get(*expression_id);
        match expression {
            _ => todo!("evaluate_type({expression:?})"),
        }

        Ok(())
    }
}
