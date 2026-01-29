use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};
use destack_source::{ModuleId, ModuleVersion};

impl Compiler {
    /// Validate module "syntactic" correctness.
    pub(crate) fn import_module_validate(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let _timing = self.timing_scope(tags::IMPORT_MODULE_VALIDATE);

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
