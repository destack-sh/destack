use crate::{Compiler, ImportResult};
use destack_source::ModuleId;

impl Compiler {
    /// Desugar a module syntactically.
    pub(crate) fn import_module_desugar_phase(&self, module: ModuleId) -> ImportResult<()> {
        self.require_import_module_bind(module)?;
        let module = self.program.modules.get(module);
        let module = module.read();
        self.bind_module_desugar(&module);
        Ok(())
    }
}
