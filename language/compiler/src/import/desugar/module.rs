use destack_dir::Expression;
use destack_source::ModuleId;

use crate::{Compiler, ImportResult};

impl Compiler {
    /// Desugar module syntactically: transforms that don't need type information:
    /// - `AssignBinary` → `Assign` + `Binary` (`x += 1` → `x = x + 1`)
    /// - `AwaitMaybe` → `Maybe` + `Await` (`await? x` → `(await x)?`)
    pub(crate) fn import_module_desugar(&self, module_id: ModuleId) -> ImportResult<()> {
        self.require_import_module_bind(module_id)?;
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
