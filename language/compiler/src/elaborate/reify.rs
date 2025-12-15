use crate::{Compiler, ElaborateResult};
use destack_source::ModuleId;

impl Compiler {
    /// Reify a module.
    pub(super) fn reify_module(&self, _module_id: ModuleId) -> ElaborateResult<()> {
        Ok(()) // #Incomplete: reify the module during elaboration
    }
}
