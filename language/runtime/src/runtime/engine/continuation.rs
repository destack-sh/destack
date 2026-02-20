use destack_vm as vm;

/// Native continuation handle for event loop integration.
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

/// Runnable continuation owned by the runtime engine.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation payloads
pub enum EngineContinuation {
    /// VM continuation that resumes MIR execution.
    Vm(vm::Continuation),
    /// Native continuation that resumes compiled execution.
    Native(NativeContinuation),
}
