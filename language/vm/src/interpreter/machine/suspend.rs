use crate::diagnostic::Error;
use crate::executable::Executable;
use crate::interpreter::{Continuation, Interpreter, YieldState};

impl Interpreter {
    /// Return an error when one captured frame still owns unsuspendable state.
    pub(crate) fn ensure_suspendable_state(
        &self,
        _executable: &Executable,
        _yield_state: &YieldState,
    ) -> Result<(), Error> {
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
