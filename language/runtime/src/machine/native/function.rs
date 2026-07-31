use std::fmt;
use std::ops::Range;

use destack_native::abi;
use destack_program::{FunctionId, Word};

/// One process-local native function.
#[derive(Clone)]
pub struct Function {
    /// The Program function implemented by this native function.
    pub function: FunctionId,
    /// The executable internal body range when caller reconstruction is available.
    body: Option<Range<usize>>,
    /// The canonical runtime entry function.
    entry: abi::Entry,
}

impl Function {
    /// Create one process-local native function.
    pub fn new(function: FunctionId, entry: abi::Entry) -> Self {
        Self {
            function,
            body: None,
            entry,
        }
    }

    /// Set the executable internal body range.
    pub fn body(mut self, body: Range<usize>) -> Self {
        self.body = Some(body);

        self
    }

    /// Return whether this function supports physical caller reconstruction.
    pub fn is_reconstructable(&self) -> bool {
        self.body.is_some()
    }

    /// Return the body-relative byte offset of one return address.
    pub fn return_offset(&self, address: usize) -> Option<u32> {
        let body = self.body.as_ref()?;
        let offset = address.checked_sub(body.start)?;
        let offset = u32::try_from(offset).ok()?;

        body.contains(&address).then_some(offset)
    }

    /// Call this native function.
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

impl fmt::Debug for Function {
    /// Format this function without exposing executable addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Function")
            .field("function", &self.function)
            .finish()
    }
}
