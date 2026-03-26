use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// Native continuation handle for event loop integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NativeContinuationHandle(usize);

impl NativeContinuationHandle {
    /// Create a new native continuation handle.
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Return the raw continuation handle value.
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Live runnable continuation owned by one backend engine.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation payloads
pub enum LiveContinuation {
    /// VM continuation that resumes MIR execution.
    Vm(vm::Continuation),
    /// Native continuation that resumes compiled execution.
    Native(NativeContinuationHandle),
}
