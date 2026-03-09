use destack_vm as vm;
use serde::{Deserialize, Serialize};

/// Native continuation handle for event loop integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NativeContinuation(usize);

impl NativeContinuation {
    /// Create a new native continuation handle.
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Return the raw continuation handle value.
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Immutable continuation image owned by one engine backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation images
pub enum EngineContinuationImage {
    /// Immutable VM continuation image.
    Vm(vm::snapshot::ContinuationImage),
    /// Native continuation handle captured for one local-only restore.
    Native(NativeContinuation),
}

/// Runnable continuation owned by one agent engine.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation payloads
pub enum EngineContinuation {
    /// VM continuation that resumes MIR execution.
    Vm(vm::Continuation),
    /// Native continuation that resumes compiled execution.
    Native(NativeContinuation),
}
