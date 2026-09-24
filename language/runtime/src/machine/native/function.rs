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
    /// The internal body offset inside the linked code image.
    body_offset: u32,
    /// The canonical runtime entry function.
    entry: abi::Entry,
}

impl Function {
    /// Create one process-local native function.
    pub fn new(function: FunctionId, entry: abi::Entry) -> Self {
        Self {
            function,
            body: None,
            body_offset: 0,
            entry,
        }
    }

    /// Set the executable internal body range.
    pub fn body(mut self, body: Range<usize>, offset: u32) -> Self {
        self.body = Some(body);
        self.body_offset = offset;

        self
    }

    /// Return whether this function supports physical caller reconstruction.
    pub fn is_reconstructable(&self) -> bool {
        self.body.is_some()
    }

    /// Return the typed native body address when available.
    pub fn body_address(&self) -> Option<usize> {
        self.body.as_ref().map(|body| body.start)
    }

    /// Return the linked code offset of one return address.
    pub fn code_offset(&self, address: usize) -> Option<u32> {
        let body = self.body.as_ref()?;
        let offset = address.checked_sub(body.start)?;
        let offset = u32::try_from(offset).ok()?;

        body.contains(&address)
            .then(|| self.body_offset.checked_add(offset))
            .flatten()
    }

    /// Call this native function.
    ///
    /// # Safety
    ///
    /// The activation must be live and describe the running program for the whole call.
    pub unsafe fn call(
        &self,
        activation: *mut abi::Activation,
        arguments: &[Word],
        result: &mut [Word],
    ) {
        // SAFETY: native entries are produced by the native linker with this ABI
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
