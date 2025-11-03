use dyst_dir::{Expression, NodeId};

use crate::{Compiler, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Expression.
    pub fn resolve_expression(&mut self, expression_id: NodeId<Expression>) -> ResolveResult<()> {
        let _expression = self.tree.get(expression_id);
        // todo!("resolve_expression({expression:?})")
        Ok(())
    }
}
