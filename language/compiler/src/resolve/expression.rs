use dyst_dir::{Expression, ModuleId, NodeId};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Expression.
    pub fn resolve_expression(
        &mut self,
        _module_id: ModuleId,
        expression_id: NodeId<Expression>,
    ) -> ResolveResult<()> {
        let _expression = self.session.tree.get(expression_id);
        Err(ResolveError::UnsupportedNode {
            node: expression_id.into(),
        })
    }
}
