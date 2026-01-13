use crate::{Compiler, ImportResult};
use destack_source::ModuleId;

impl Compiler {
    /// Validate module "syntactic" correctness.
    pub(crate) fn import_module_validate(&self, module_id: ModuleId) -> ImportResult<()> {
        self.require_import_module_desugar(module_id)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let module = module.read();
        self.validate_binding_names(&module);
        self.validate_binding_conflicts(&module);
        Ok(())
    }
}
