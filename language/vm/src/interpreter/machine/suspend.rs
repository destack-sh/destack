use destack_engine as engine;
use destack_heap::{Value, ValueTag};

use crate::diagnostic::Error;
use crate::executable::Executable;
use crate::execute::{Continuation, YieldState, frame_capture_materialization};
use crate::interpreter::{Frame, Interpreter};

impl Interpreter {
    /// Return whether one value still points into frame-local storage.
    fn is_frame_local_suspend_value(value: Value) -> bool {
        matches!(value.tag(), ValueTag::StackPointer | ValueTag::LocalPointer)
    }

    /// Return whether one frame still exposes frame-local pointers through materialized slots.
    fn frame_has_live_suspend_pointers(
        frame: &Frame,
        value_stack: &[Value],
        local_stack: &[Value],
        materialization_frame: &engine::MaterializationFrame,
    ) -> bool {
        // slice the live stacks for this frame once before scanning slots
        let value_slice = &value_stack[frame.value_base..frame.value_base + frame.value_count];
        let local_slice = &local_stack[frame.local_base..frame.local_base + frame.local_count];

        // reject any materialized slot that still exposes frame-local pointers
        for slot in &materialization_frame.slots {
            let engine::MaterializationValue::FrameSlot(slot_index) = slot.value else {
                continue;
            };

            let value = frame
                .slot_value(value_slice, local_slice, slot_index)
                .unwrap_or_else(|| {
                    panic!(
                        "missing frame slot during suspend validation: {:?} {slot_index}",
                        frame.function
                    )
                });

            if Self::is_frame_local_suspend_value(value) {
                return true;
            }
        }

        false
    }

    /// Return an error when one captured frame still owns frame-local suspend state.
    pub(crate) fn ensure_suspendable_state(
        &self,
        executable: &Executable,
        yield_state: &YieldState,
    ) -> Result<(), Error> {
        // reject live dynamic stack-local storage across suspension
        for (frame_index, frame) in self.call_stack.iter().enumerate() {
            if frame.has_live_stack_allocations() {
                return Err(Error::SuspendWithFrameLocalState);
            }

            // reject materialized frame-local pointers that would escape suspension
            let (_, materialization_frame) =
                frame_capture_materialization(executable, frame, frame_index, yield_state);
            if Self::frame_has_live_suspend_pointers(
                frame,
                &self.value_stack,
                &self.local_stack,
                materialization_frame,
            ) {
                return Err(Error::SuspendWithFrameLocalState);
            }
        }

        Ok(())
    }

    /// Capture execution state into a continuation.
    pub(crate) fn capture_continuation(
        &mut self,
        isolate_id: u64,
        yield_state: YieldState,
    ) -> Continuation {
        // move execution stacks into the continuation
        let call_stack = std::mem::take(&mut self.call_stack);
        let value_stack = std::mem::take(&mut self.value_stack);
        let local_stack = std::mem::take(&mut self.local_stack);

        // move execution statistics into the continuation
        let statistics = std::mem::take(&mut self.statistics);

        // move instruction profile state into the continuation
        #[cfg(feature = "stats")]
        let instruction_profile = self.instruction_profile.take();

        Continuation {
            isolate_id,
            call_stack,
            value_stack,
            local_stack,
            yield_state,
            statistics,
            #[cfg(feature = "stats")]
            instruction_profile,
        }
    }
}
