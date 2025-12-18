use crate::{Compiler, ElaborateResult};
use destack_source::ModuleId;

impl Compiler {
    /// Reify a module: make abstract constructs concrete.
    ///
    /// Transforms:
    ///  - nocheckin???
    /// - `Type<T>` runtime values → type descriptor generation
    pub(super) fn reify_module(&self, _module_id: ModuleId) -> ElaborateResult<()> {
        Ok(()) // TODO #Incomplete: reify the module during elaboration
    }
}
