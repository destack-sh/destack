use destack_heap::Value;
use serde::{Deserialize, Serialize};

use crate::snapshot::FrameSnapshot;
use crate::telemetry::Statistics;

/// Durable interpreter state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterSnapshot {
    /// The captured call stack.
    pub call_stack: Vec<FrameSnapshot>,
    /// The captured SSA value stack.
    pub value_stack: Vec<Value>,
    /// The captured local stack.
    pub local_stack: Vec<Value>,
    /// The captured execution statistics.
    pub statistics: Statistics,
}
