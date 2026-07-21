use std::ffi::c_void;
use std::fmt;
use std::ptr::NonNull;

use crate::ProgramStorage;

/// One call from the runtime into a program machine.
pub struct ProgramActivation<'a> {
    /// Runtime-owned call state.
    pub state: NonNull<c_void>,
    /// Memory available to this call.
    pub storage: ProgramStorage<'a>,
}

impl ProgramActivation<'_> {
    /// Reborrow this activation for one nested machine call.
    pub fn reborrow(&mut self) -> ProgramActivation<'_> {
        ProgramActivation {
            state: self.state,
            storage: self.storage.reborrow(),
        }
    }
}

impl fmt::Debug for ProgramActivation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProgramActivation")
            .field("state", &true)
            .field("storage", &self.storage)
            .finish()
    }
}
