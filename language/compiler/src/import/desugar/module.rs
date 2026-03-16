use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};
use destack_dir::Expression;
use destack_source::{ModuleId, ModuleVersion};
use destack_workspace::ImportDir;

impl Compiler {
    /// Desugar module syntactically: transforms that don't need type information:
    /// - `AssignBinary` → `Assign` + `Binary` (`x += 1` → `x = x + 1`)
    /// - `AwaitMaybe` → `Maybe` + `Await` (`await? x` → `(await x)?`)
    pub(crate) fn import_module_desugar(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        dir: &mut ImportDir,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let _timing = self.timing_scope(tags::IMPORT_MODULE_DESUGAR);

        // syntax-only modules stop at AST
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // desugar expressions
        let expression_ids = dir.tree.iter_node_ids_of_type::<Expression>();
        for expression_id in expression_ids {
            self.desugar_expression(expression_id, &mut dir.tree);
        }
        Ok(())
    }
}
