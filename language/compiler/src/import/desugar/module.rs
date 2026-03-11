use destack_dir::Expression;
use destack_source::{ModuleId, ModuleVersion};

use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};

impl Compiler {
    /// Desugar module syntactically: transforms that don't need type information:
    /// - `AssignBinary` → `Assign` + `Binary` (`x += 1` → `x = x + 1`)
    /// - `AwaitMaybe` → `Maybe` + `Await` (`await? x` → `(await x)?`)
    pub(crate) fn import_module_desugar(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let _timing = self.timing_scope(tags::IMPORT_MODULE_DESUGAR);

        // syntax-only modules stop at AST
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let module = module.read();
        let mut tree = module.dir_base().tree.write();

        // desugar expressions
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            self.desugar_expression(expression_id, &mut tree);
        }
        Ok(())
    }
}
