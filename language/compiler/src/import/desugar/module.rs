use crate::{Compiler, ImportResult};
use destack_dir::{Expression, Tree};
use destack_source::ModuleId;
use destack_workspace::ProviderContext;

impl Compiler {
    /// Desugar module syntactically: transforms that don't need type information:
    /// - `AssignBinary` → `Assign` + `Binary` (`x += 1` → `x = x + 1`)
    /// - `AwaitMaybe` → `Maybe` + `Await` (`await? x` → `(await x)?`)
    pub(crate) fn import_module_desugar(
        &self,
        module_id: ModuleId,
        tree: &mut Tree,
        context: &dyn ProviderContext,
    ) -> ImportResult<()> {
        // syntax-only modules stop at AST
        if !self.is_code_module(context.revision(), module_id) {
            return Ok(());
        }

        // desugar expressions
        let expression_ids = tree.iter_node_ids_of_type::<Expression>();
        for expression_id in expression_ids {
            self.desugar_expression(expression_id, tree);
        }
        Ok(())
    }
}
