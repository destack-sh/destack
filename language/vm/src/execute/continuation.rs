use destack_mir as mir;

use crate::interpreter::{CopyRange, Frame};
use crate::memory::{HeapHandle, Value};
#[cfg(feature = "stats")]
use crate::telemetry::InstructionProfile;
use crate::telemetry::Statistics;

/// Resume state captured at a yield terminator.
#[derive(Debug, Clone)]
pub(crate) struct YieldState {
    /// Frame index to resume execution in.
    pub frame_index: usize,
    /// Resume block index in the threaded function.
    pub resume_block: u32,
    /// Copy plan for resume arguments.
    pub resume_copies: CopyRange,
    /// Destination for the resumed value.
    pub resume_value: mir::Value,
}

/// Continuation snapshot captured at a yield terminator.
#[derive(Debug)]
pub struct Continuation {
    /// The isolate id used to validate the continuation.
    pub(crate) isolate_id: u64,
    /// The call stack for the suspended execution.
    pub(crate) call_stack: Vec<Frame>,
    /// The SSA value stack for the suspended execution.
    pub(crate) value_stack: Vec<Value>,
    /// The local variable stack for the suspended execution.
    pub(crate) local_stack: Vec<Value>,
    /// The resume state captured at the yield point.
    pub(crate) yield_state: YieldState,
    /// The statistics captured for the suspended execution.
    pub(crate) statistics: Statistics,
    /// The instruction profile state for the suspended execution.
    #[cfg(feature = "stats")]
    pub(crate) instruction_profile: Option<InstructionProfile>,
}

impl Continuation {
    /// Clone this continuation for multi-shot resumption.
    pub fn clone_for_fork(&self) -> Self {
        let call_stack = self.call_stack.iter().map(Frame::clone_for_fork).collect();
        let value_stack = self.value_stack.clone();
        let local_stack = self.local_stack.clone();
        let yield_state = self.yield_state.clone();
        let statistics = self.statistics.clone();
        #[cfg(feature = "stats")]
        let instruction_profile = self.instruction_profile.clone();

        Self {
            isolate_id: self.isolate_id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics,
            #[cfg(feature = "stats")]
            instruction_profile,
        }
    }

    /// Collect managed heap roots referenced by this continuation.
    pub fn collect_roots(&self, roots: &mut Vec<HeapHandle>) {
        // collect roots from captured frames
        for frame in &self.call_stack {
            frame.collect_roots(&self.value_stack, &self.local_stack, roots);
        }
    }
}
