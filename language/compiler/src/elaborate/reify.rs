use crate::{Compiler, ElaborateResult};
use destack_source::ModuleId;

impl Compiler {
    /// Reify a module: make abstract constructs concrete (needs type info).
    ///
    /// Transforms:
    /// - Pattern matching → nested if/else chains with destructuring
    /// - `match` expressions → conditional chains
    /// - `if` expressions (as values) → temp variable pattern
    /// - `Type<T>` runtime values → type descriptor generation
    pub(super) fn reify_module(&self, _module_id: ModuleId) -> ElaborateResult<()> {
        Ok(()) // TODO #Incomplete: reify the module during elaboration
    }
}
