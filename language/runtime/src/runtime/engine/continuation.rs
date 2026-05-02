use serde::{Deserialize, Serialize};
use {destack_engine as engine, destack_native as native, destack_vm as vm};

/// Live runnable continuation owned by one backend engine.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation payloads
pub enum Continuation {
    /// VM continuation that resumes MIR execution.
    Vm(vm::Continuation),
    /// Native continuation that resumes compiled execution.
    Native(native::Continuation),
}

/// Durable continuation image owned by one backend engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuationImage {
    /// VM continuation image.
    Vm(vm::ContinuationImage),
    /// Native continuation image.
    Native(engine::Continuation),
}
