use destack_heap::Value;
use serde::{Deserialize, Serialize};

use crate::snapshot::InterpreterFrameImage;
use crate::telemetry::Statistics;

/// Immutable interpreter image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterImage {
    /// The captured call stack.
    pub call_stack: Vec<InterpreterFrameImage>,
    /// The captured SSA value stack.
    pub value_stack: Vec<Value>,
    /// The captured local stack.
    pub local_stack: Vec<Value>,
    /// The captured execution statistics.
    pub statistics: Statistics,
}
