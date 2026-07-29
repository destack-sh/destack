use std::fmt;
use std::sync::Arc;

use destack_memory::MemoryMap;
use destack_program as program;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::{Machine, MachineState, native};
use crate::diagnostic::{MachineError, MachineKind, RuntimeError, RuntimeResult};

/// Immutable execution engine shared by one runtime.
#[derive(Clone)]
pub enum Engine {
    /// Interpret Program bytecode.
    Vm {
        /// VM machine limits.
        limits: vm::MachineLimits,
    },
    /// Execute process-local native code.
    Native {
        /// Loaded native code.
        code: Arc<native::Code>,
    },
}

/// Captured configuration for one runtime execution engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineImage {
    /// Interpret Program bytecode.
    Vm {
        /// VM machine limits.
        limits: vm::MachineLimits,
    },
    /// Execute process-local native code.
    Native,
}

impl Engine {
    /// Create a bytecode interpreter.
    pub const fn vm(limits: vm::MachineLimits) -> Self {
        Self::Vm { limits }
    }

    /// Create a native execution engine.
    pub fn native(code: native::Code) -> Self {
        Self::Native {
            code: Arc::new(code),
        }
    }

    /// Capture this runtime execution engine configuration.
    pub const fn image(&self) -> EngineImage {
        match self {
            Self::Vm { limits } => EngineImage::Vm { limits: *limits },
            Self::Native { .. } => EngineImage::Native,
        }
    }

    /// Restore an execution engine from one captured runtime.
    pub fn restore(
        program: &program::Program,
        image: EngineImage,
        loader: Option<&dyn native::Loader>,
    ) -> RuntimeResult<Self> {
        match image {
            EngineImage::Vm { limits } => Ok(Self::vm(limits)),
            EngineImage::Native => {
                let Some(loader) = loader else {
                    return Err(RuntimeError::machine(
                        MachineKind::Native,
                        MachineError::Unsupported {
                            feature: "image restore without native loader".to_string(),
                        },
                    )
                    .boxed());
                };
                let code = loader.load(program).map_err(Box::<RuntimeError>::from)?;

                Ok(Self::native(code))
            }
        }
    }

    /// Spawn one worker-owned machine.
    pub fn spawn(
        &self,
        program: Arc<program::Program>,
        memory: Arc<MemoryMap>,
    ) -> RuntimeResult<Machine> {
        let state = match self {
            Self::Vm { limits } => {
                let machine = vm::Machine::new(program, memory, *limits)
                    .map_err(Box::<RuntimeError>::from)?;

                MachineState::Vm(Box::new(machine))
            }
            Self::Native { code } => {
                MachineState::Native(native::Machine::new(program, code.clone()))
            }
        };

        Ok(Machine::from_state(state))
    }
}

impl fmt::Debug for Engine {
    /// Format the engine without exposing executable internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vm { limits } => formatter
                .debug_struct("Vm")
                .field("limits", limits)
                .finish(),
            Self::Native { code } => formatter
                .debug_struct("Native")
                .field("code", code)
                .finish(),
        }
    }
}
