use std::ffi::c_void;
use std::fmt;
use std::ptr::NonNull;

use crate::Memory;

/// One active program execution.
pub struct Activation<'a> {
    /// Opaque runtime context available to binding calls.
    pub context: NonNull<c_void>,
    /// Memory available to this call.
    pub memory: Memory<'a>,
}

impl Activation<'_> {
    /// Reborrow this activation for one nested machine call.
    pub fn reborrow(&mut self) -> Activation<'_> {
        Activation {
            context: self.context,
            memory: self.memory.reborrow(),
        }
    }
}

impl fmt::Debug for Activation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Activation")
            .field("context", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
