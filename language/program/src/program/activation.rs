use std::ffi::c_void;
use std::fmt;
use std::ptr::NonNull;

use crate::{Memory, Result, Value, Waiter};

/// Runtime services available to executing program code.
pub trait Runtime {
    /// Return the opaque runtime state passed through the native ABI.
    fn native_state(&mut self) -> NonNull<c_void>;

    /// Queue one suspended waiter with its result value.
    fn queue_waiter(&mut self, waiter: Waiter, value: Value) -> Result<()>;

    /// Cancel one suspended waiter through its cleanup path.
    fn cancel_waiter(&mut self, waiter: Waiter) -> Result<()>;
}

/// One active program execution.
pub struct Activation<'runtime, 'memory> {
    /// Runtime operations available to executing code.
    pub runtime: &'runtime mut dyn Runtime,
    /// Memory available to this call.
    pub memory: Memory<'memory>,
}

impl Activation<'_, '_> {
    /// Reborrow this activation for one nested machine call.
    pub fn reborrow(&mut self) -> Activation<'_, '_> {
        Activation {
            runtime: &mut *self.runtime,
            memory: self.memory.reborrow(),
        }
    }
}

impl fmt::Debug for Activation<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Activation")
            .field("runtime", &true)
            .field("memory", &self.memory)
            .finish()
    }
}
