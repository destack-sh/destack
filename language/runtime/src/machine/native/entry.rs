use std::fmt;

use destack_native::abi;
use destack_program::{FunctionId, Word};

/// One native entry.
#[derive(Clone)]
pub struct Entry {
    /// The function implemented by this entry.
    pub function: FunctionId,
    /// The native entry function pointer.
    pub entry: abi::Entry,
}

impl Entry {
    /// Create one native entry.
    pub fn new(function: FunctionId, entry: abi::Entry) -> Self {
        Self { function, entry }
    }

    /// Call this native entry.
    pub fn call(
        &self,
        activation: &mut abi::Activation,
        arguments: &[Word],
        result: &mut [Word],
    ) -> abi::ExitCode {
        // native entries are produced by the native linker with this ABI
        unsafe {
            (self.entry)(
                activation,
                arguments.as_ptr().cast(),
                result.as_mut_ptr().cast(),
            )
        }
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
