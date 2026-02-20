use destack_vm as vm;

use super::{Microtask, Task, Timer};
use crate::platform::PlatformEvent;

/// Runnable item returned by the scheduler.
#[derive(Debug)]
pub enum Runnable {
    /// A macrotask selected for execution.
    Task(Task),
    /// A microtask selected for execution.
    Microtask(Microtask),
    /// A timer ready to fire.
    Timer(Timer),
    /// An external platform event.
    Event(PlatformEvent),
}

/// Native continuation handle for scheduler integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeContinuation(u64);

impl NativeContinuation {
    /// Create a new native continuation handle.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw continuation handle value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Runnable continuation owned by the scheduler.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime::Runnable enum
pub enum PlatformRunnable {
    /// VM continuation that resumes MIR execution.
    Vm(vm::Continuation),
    /// Native continuation that resumes compiled execution.
    Native(NativeContinuation),
}
