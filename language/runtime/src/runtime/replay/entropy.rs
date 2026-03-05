use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{EntropyEvent, EntropyKind, EntropySubject, Replay, ReplayEvent};
use destack_workspace::ExecutionMode;

/// Replay channel name for entropy events.
const ENTROPY_CHANNEL: &str = "entropy";

impl Replay {
    /// Return one replay mismatch error for the entropy channel.
    pub(crate) fn entropy_mismatch_error(&self) -> Box<RuntimeError> {
        RuntimeError::ReplayMismatch {
            name: ENTROPY_CHANNEL.to_string(),
        }
        .boxed()
    }

    /// Read and validate one entropy event for replay.
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
            return Err(RuntimeError::ReplayLogExhausted { sequence }.boxed());
        };

        // validate replay event channel and key
        let ReplayEvent::Entropy(entropy_event) = event else {
            return Err(self.entropy_mismatch_error());
        };
        if entropy_event.kind() != expected_kind {
            return Err(self.entropy_mismatch_error());
        }

        // validate replay subject identity
        if entropy_event.subject() != expected_subject {
            return Err(self.entropy_mismatch_error());
        }

        Ok(entropy_event)
    }
}
