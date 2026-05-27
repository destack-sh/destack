use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::trace::{EntropySample, EntropySubject, Outcome, Trace, TraceError, TraceRecord};
use destack_workspace::ExecutionMode;

impl Trace {
    /// Run one monotonic-clock read binding through the entropy replay channel.
    pub fn run_time_monotonic_read<Hook, Call>(
        &self,
        subject: EntropySubject,
        on_replay_read: Hook,
        call: Call,
    ) -> RuntimeResult<u64>
    where
        Hook: FnOnce(),
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        self.run_time_read(
            subject,
            on_replay_read,
            call,
            |subject, outcome| EntropySample::TimeReadMonotonic { subject, outcome },
            |sample| match sample {
                EntropySample::TimeReadMonotonic { outcome, .. } => Some(outcome),
                _ => None,
            },
        )
    }

    /// Run one wall-clock read binding through the entropy replay channel.
    pub fn run_time_wall_read<Hook, Call>(
        &self,
        subject: EntropySubject,
        on_replay_read: Hook,
        call: Call,
    ) -> RuntimeResult<u64>
    where
        Hook: FnOnce(),
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        self.run_time_read(
            subject,
            on_replay_read,
            call,
            |subject, outcome| EntropySample::TimeReadWall { subject, outcome },
            |sample| match sample {
                EntropySample::TimeReadWall { outcome, .. } => Some(outcome),
                _ => None,
            },
        )
    }

    /// Run one clock read binding through the entropy replay channel.
    fn run_time_read<Hook, Call, Record, Replay>(
        &self,
        subject: EntropySubject,
        on_replay_read: Hook,
        call: Call,
        record: Record,
        replay: Replay,
    ) -> RuntimeResult<u64>
    where
        Hook: FnOnce(),
        Call: FnOnce() -> RuntimeResult<u64>,
        Record: FnOnce(EntropySubject, Result<u64, TraceError>) -> EntropySample,
        Replay: FnOnce(EntropySample) -> Option<Result<u64, TraceError>>,
    {
        let mode = self.mode();

        match mode {
            // fast and strict modes execute directly
            ExecutionMode::Fast | ExecutionMode::Strict => call(),
            // replay mode reads one recorded entropy sample
            ExecutionMode::Replay => {
                on_replay_read();
                let event = self.next_entropy_sample(subject)?;

                match replay(event) {
                    Some(outcome) => outcome.map_err(Box::<RuntimeError>::from),
                    None => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one entropy sample
            ExecutionMode::Record => {
                let result = call();
                let outcome = result
                    .as_ref()
                    .map(|value| *value)
                    .map_err(|error| TraceError::from(error.as_ref()));
                let event = record(subject, outcome);
                self.record_event(TraceRecord::Outcome(Outcome::Entropy(event)))?;

                result
            }
        }
    }
}
