use std::fmt;

use destack_program::native::{NativeContext, NativeEntry, NativeExitCode};
use destack_program::{FunctionId, Word};

/// One native entry.
#[derive(Clone)]
pub struct Entry {
    /// The function implemented by this entry.
    pub function: FunctionId,
    /// The native entry function pointer.
    pub entry: NativeEntry,
}

impl Entry {
    /// Create one native entry.
    pub fn new(function: FunctionId, entry: NativeEntry) -> Self {
        Self { function, entry }
    }

    /// Call this native entry.
    pub fn call(
        &self,
        context: &mut NativeContext,
        arguments: &[Word],
        result: &mut [Word],
    ) -> NativeExitCode {
        // native entries are produced by the native linker with this ABI
        unsafe { (self.entry)(context, arguments.as_ptr(), result.as_mut_ptr()) }
    }
}

impl fmt::Debug for Entry {
    /// Format this entry without exposing a raw function pointer.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Entry")
            .field("function", &self.function)
            .finish()
    }
}
