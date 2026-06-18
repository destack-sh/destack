use destack_native as native;
use destack_program as program;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::ExecutorId;

/// Live runnable continuation owned by one executor.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation payloads
pub enum Continuation {
    /// VM continuation that resumes MIR execution.
    Vm {
        /// Executor that owns the continuation.
        executor: ExecutorId,
        /// VM continuation that resumes MIR execution.
        continuation: vm::Continuation,
    },
    /// Native continuation that resumes compiled execution.
    Native {
        /// Executor that owns the continuation.
        executor: ExecutorId,
        /// Native continuation that resumes compiled execution.
        continuation: native::Continuation,
    },
}

/// Durable continuation image owned by one executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuationImage {
    /// VM continuation image.
    Vm {
        /// Executor that owns the continuation image.
        executor: ExecutorId,
        /// VM continuation image.
        image: vm::ContinuationImage,
    },
    /// Native continuation image.
    Native {
        /// Executor that owns the continuation image.
        executor: ExecutorId,
        /// Native continuation image.
        image: program::MaterializedContinuation,
    },
}

impl Continuation {
    /// Return the executor that owns this continuation.
    pub const fn executor(&self) -> ExecutorId {
        match self {
            Self::Vm { executor, .. } | Self::Native { executor, .. } => *executor,
        }
    }
}

impl ContinuationImage {
    /// Return the executor that owns this continuation image.
    pub const fn executor(&self) -> ExecutorId {
        match self {
            Self::Vm { executor, .. } | Self::Native { executor, .. } => *executor,
        }
    }
}
