use destack_mir as mir;
use serde::{Deserialize, Serialize};

use crate::snapshot::FrameImage;
use crate::telemetry::Statistics;
use destack_heap::Value;

/// Immutable resume state captured at one yield point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YieldStateImage {
    /// Frame index to resume execution in.
    pub frame_index: usize,
    /// Resume block index in the current function.
    pub resume_block: u32,
    /// Copy-range start offset.
    pub resume_copy_start: u32,
    /// Copy-range length.
    pub resume_copy_len: u32,
    /// Copy-range contiguous marker.
    pub resume_copy_is_contiguous: bool,
    /// First contiguous source id when present.
    pub resume_copy_contiguous_src: u32,
    /// First contiguous destination id when present.
    pub resume_copy_contiguous_dest: u32,
    /// Destination for the resumed value.
    pub resume_value: mir::Value,
}

/// Immutable continuation image for one suspended execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationImage {
    /// The isolate id used to validate this continuation.
    pub isolate_id: u64,
    /// The captured call stack.
    pub call_stack: Vec<FrameImage>,
    /// The captured SSA value stack.
    pub value_stack: Vec<Value>,
    /// The captured local variable stack.
    pub local_stack: Vec<Value>,
    /// The captured resume state.
    pub yield_state: YieldStateImage,
    /// The captured execution statistics.
    pub statistics: Statistics,
}
