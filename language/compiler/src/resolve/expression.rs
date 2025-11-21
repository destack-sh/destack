use dyst_dir::{Expression, ModuleId, NodeId, NodeTree};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve an Expression.
    pub(super) fn resolve_expression(
        &self,
        _module_id: ModuleId,
        expression_id: NodeId<Expression>,
        tree: &mut NodeTree,
    ) -> ResolveResult<()> {
        let expression = tree.get(expression_id);
        Ok(())
    }
}
