use crate::diagnostic::Error;
use crate::interpreter::{Continuation, Interpreter};
use crate::module::Module;
use destack_engine as engine;
use destack_engine::ResumePointId;

impl Interpreter {
    /// Return an error when one captured frame still owns unsuspendable state.
    pub(crate) fn ensure_suspendable_state(
        &self,
        _module: &Module,
        _resume_frame_index: usize,
        _resume_point: ResumePointId,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Capture execution state into a continuation.
    pub(crate) fn capture_continuation(
        &mut self,
        isolate_id: engine::IsolateId,
        resume_frame_index: usize,
        resume_point: ResumePointId,
    ) -> Continuation {
        // move execution stack into the continuation
        let stack = std::mem::take(&mut self.stack);

        // move execution statistics into the continuation
        let statistics = std::mem::take(&mut self.statistics);

        // move instruction profile state into the continuation
        #[cfg(feature = "stats")]
        let instruction_profile = self.instruction_profile.take();

        Continuation {
            isolate_id,
            stack,
            resume_frame_index,
            resume_point,
            statistics,
            #[cfg(feature = "stats")]
            instruction_profile,
        }
    }
}
