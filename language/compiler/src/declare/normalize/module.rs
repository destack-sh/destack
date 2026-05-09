use crate::{Compiler, DeclareResult};
use destack_dir::{Expression, Tree};
use destack_source::ModuleId;
use destack_workspace::ProviderContext;

impl Compiler {
    /// Normalize one declared module without type information.
    pub(crate) fn declare_module_normalize(
        &self,
        module_id: ModuleId,
        tree: &mut Tree,
        context: &dyn ProviderContext,
    ) -> DeclareResult<()> {
        // syntax-only modules stop at AST
        if !self.is_code_module(context.revision(), module_id) {
            return Ok(());
        }

        // normalize expressions
        let expression_ids = tree.iter_node_ids_of_type::<Expression>();
        for expression_id in expression_ids {
            self.normalize_expression(expression_id, tree);
        }
        Ok(())
    }
}
