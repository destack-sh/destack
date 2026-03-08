use serde::{Deserialize, Serialize};

/// Execution role for a VM isolate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Evaluate compile time blocks and expressions.
    #[default]
    Comptime,
    /// Run under the debugger with introspection enabled.
    Debug,
    /// Resume execution after deoptimization from native code.
    Deopt,
    /// Execute as a general runtime fallback.
    Runtime,
}

impl ExecutionMode {
    /// Report whether this mode should enable debug checks.
    pub fn is_debug(self) -> bool {
        matches!(self, Self::Debug)
    }
}

/// Execution mode options for an isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// The execution role for this isolate.
    pub mode: ExecutionMode,
}
