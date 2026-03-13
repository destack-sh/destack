use destack_mir as mir;

use crate::diagnostic::RuntimeResult;
use crate::interpreter::{CopyRange, Frame, ThreadedFunctionTable};
use crate::snapshot::{ContinuationImage, YieldStateImage};
#[cfg(feature = "stats")]
use crate::telemetry::InstructionProfile;
use crate::telemetry::Statistics;
use destack_heap::{ManagedReference, Value};

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
        // clone all stack state for the fork
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

    /// Capture one immutable continuation image.
    pub fn image(&self) -> ContinuationImage {
        // capture the current stack state
        let call_stack = self.call_stack.iter().map(Frame::image).collect();
        let value_stack = self.value_stack.clone();
        let local_stack = self.local_stack.clone();

        // capture the yield metadata
        let yield_state = YieldStateImage {
            frame_index: self.yield_state.frame_index,
            resume_block: self.yield_state.resume_block,
            resume_copy_start: self.yield_state.resume_copies.start,
            resume_copy_len: self.yield_state.resume_copies.len,
            resume_copy_is_contiguous: self.yield_state.resume_copies.is_contiguous,
            resume_copy_contiguous_src: self.yield_state.resume_copies.contiguous_src,
            resume_copy_contiguous_dest: self.yield_state.resume_copies.contiguous_dest,
            resume_value: self.yield_state.resume_value,
        };

        ContinuationImage {
            isolate_id: self.isolate_id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics: self.statistics.clone(),
        }
    }

    /// Collect managed heap roots referenced by this continuation.
    pub fn collect_roots(&self, roots: &mut Vec<ManagedReference>) {
        // collect roots from captured frames
        for frame in &self.call_stack {
            frame.collect_roots(&self.value_stack, &self.local_stack, roots);
        }
    }

    /// Rebuild one continuation from an immutable image.
    pub(crate) fn from_image(
        image: &ContinuationImage,
        threaded_functions: &ThreadedFunctionTable,
    ) -> RuntimeResult<Self> {
        // rebuild the captured stack state
        let call_stack = image
            .call_stack
            .iter()
            .map(|frame| Frame::from_image(frame, threaded_functions))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let value_stack = image.value_stack.clone();
        let local_stack = image.local_stack.clone();

        // rebuild the yield metadata
        let yield_state = YieldState {
            frame_index: image.yield_state.frame_index,
            resume_block: image.yield_state.resume_block,
            resume_copies: CopyRange {
                start: image.yield_state.resume_copy_start,
                len: image.yield_state.resume_copy_len,
                is_contiguous: image.yield_state.resume_copy_is_contiguous,
                contiguous_src: image.yield_state.resume_copy_contiguous_src,
                contiguous_dest: image.yield_state.resume_copy_contiguous_dest,
            },
            resume_value: image.yield_state.resume_value,
        };

        Ok(Self {
            isolate_id: image.isolate_id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics: image.statistics.clone(),
            #[cfg(feature = "stats")]
            instruction_profile: None,
        })
    }
}
