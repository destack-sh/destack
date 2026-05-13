use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::trace::{
    EntropyEvent, EntropyKind, EntropySubject, Outcome, Trace, TraceError, TraceRecord,
};
use destack_workspace::ExecutionMode;

impl Trace {
    /// Run one clock read binding through the entropy replay channel.
    pub fn run_time_read<Hook, Call>(
        &self,
        kind: EntropyKind,
        subject: EntropySubject,
        on_replay_read: Hook,
        call: Call,
    ) -> RuntimeResult<u64>
    where
        Hook: FnOnce(),
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        let mode = self.mode();

        match mode {
            // fast and deterministic modes execute directly
            ExecutionMode::Fast | ExecutionMode::Deterministic => call(),
            // replay mode reads one recorded entropy sample
            ExecutionMode::Replay => {
                on_replay_read();
                let event = self.next_entropy_event(kind, subject)?;

                match event {
                    EntropyEvent::TimeReadMonotonic { outcome, .. }
                    | EntropyEvent::TimeReadWall { outcome, .. } => {
                        outcome.map_err(Box::<RuntimeError>::from)
                    }
                    _ => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one entropy sample
            ExecutionMode::Record => {
                let result = call();
                let outcome = result
                    .as_ref()
                    .map(|value| *value)
                    .map_err(|error| TraceError::from(error.as_ref()));
                let event = match kind {
                    EntropyKind::TimeReadMonotonic => {
                        EntropyEvent::TimeReadMonotonic { subject, outcome }
                    }
                    EntropyKind::TimeReadWall => EntropyEvent::TimeReadWall { subject, outcome },
                    _ => return Err(self.entropy_mismatch_error()),
                };
                self.record_event(TraceRecord::Outcome(Outcome::Entropy(event)))?;

                result
            }
        }
    }
}
