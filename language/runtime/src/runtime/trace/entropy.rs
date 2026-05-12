use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::trace::{
    EntropyEvent, EntropyKind, EntropySubject, Outcome, Trace, TraceRecord,
};
use destack_workspace::ExecutionMode;

/// Trace channel name for entropy events.
pub(super) const ENTROPY_CHANNEL: &str = "runtime.random.entropy";

impl Trace {
    /// Return one trace mismatch error for the entropy channel.
    pub(crate) fn entropy_mismatch_error(&self) -> Box<RuntimeError> {
        RuntimeError::TraceMismatch {
            name: ENTROPY_CHANNEL.to_string(),
        }
        .boxed()
    }

    /// Read and validate one entropy event from trace.
    pub(crate) fn next_entropy_event(
        &self,
        expected_kind: EntropyKind,
        expected_subject: EntropySubject,
    ) -> RuntimeResult<EntropyEvent> {
        // reject reads outside replay execution
        if self.mode() != ExecutionMode::Replay {
            return Err(self.entropy_mismatch_error());
        }

        // read the next event from the log
        let Some(event) = self.next_event()? else {
            let sequence = self.log().next_sequence().get();
            return Err(RuntimeError::TraceExhausted { sequence }.boxed());
        };

        // validate trace event channel and key
        let TraceRecord::Outcome(Outcome::Entropy(entropy_event)) = event else {
            return Err(self.entropy_mismatch_error());
        };
        if entropy_event.kind() != expected_kind {
            return Err(self.entropy_mismatch_error());
        }

        // validate trace subject identity
        if entropy_event.subject() != expected_subject {
            return Err(self.entropy_mismatch_error());
        }

        Ok(entropy_event)
    }
}
