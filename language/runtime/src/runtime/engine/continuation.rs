use {destack_native as native, destack_vm as vm};

pub use destack_engine::ContinuationImage;

/// Live runnable continuation owned by one backend engine.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation payloads
pub enum Continuation {
    /// VM continuation that resumes MIR execution.
    Vm(vm::Continuation),
    /// Native continuation that resumes compiled execution.
    Native(native::Continuation),
}
