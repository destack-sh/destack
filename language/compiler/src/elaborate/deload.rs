use crate::{Compiler, ElaborateResult};
use destack_source::ModuleId;

impl Compiler {
    /// Deload a module.
    pub(super) fn deload_module(&self, _module_id: ModuleId) -> ElaborateResult<()> {
        Ok(()) // #Incomplete: resolve overloads ("deload") the module during elaboration
    }
}
