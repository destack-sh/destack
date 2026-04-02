use destack_heap::Value;
use serde::{Deserialize, Serialize};
use std::mem::size_of;

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

impl InterpreterImage {
    /// Return the owned bytes for this durable interpreter image.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.call_stack.capacity() * size_of::<InterpreterFrameImage>();
        owned_bytes += self.value_stack.capacity() * size_of::<Value>();
        owned_bytes += self.local_stack.capacity() * size_of::<Value>();

        for frame in &self.call_stack {
            owned_bytes += frame.owned_bytes() - size_of::<InterpreterFrameImage>();
        }

        owned_bytes
    }
}
