use crate::{Compiler, ElaborateResult};
use destack_source::ModuleId;

impl Compiler {
    /// Deload a module: resolve overloaded operators to method calls.
    ///
    /// For types implementing operator interfaces, transforms:
    /// - `a + b` → `a.add(b)` (when `a: T` implements `Add`)
    /// - `a == b` → `a.equal(b)` (when `a: T` implements `Equal`)
    /// - `a < b` → `a.compare(b) < 0` (when `a: T` implements `Compare`)
    /// - `a[i]` → `a.get(i)` (when `a: T` implements `Index`)
    ///
    /// Primitives keep their operators as-is. Uses receiver-based dispatch.
    pub(super) fn deload_module(&self, _module_id: ModuleId) -> ElaborateResult<()> {
        Ok(()) // TODO #Incomplete: resolve overloads ("deload") the module during elaboration
    }
}
