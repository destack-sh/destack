use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::trace::{ClockTrace, EntropySubject, Trace, TraceError, TraceLog};
use destack_repository::ExecutionMode;

impl TraceLog {
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
            |subject, outcome| ClockTrace::ReadMonotonic { subject, outcome },
            |sample| match sample {
                ClockTrace::ReadMonotonic { outcome, .. } => Some(outcome),
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
            |subject, outcome| ClockTrace::ReadWall { subject, outcome },
            |sample| match sample {
                ClockTrace::ReadWall { outcome, .. } => Some(outcome),
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
        Record: FnOnce(EntropySubject, Result<u64, TraceError>) -> ClockTrace,
        Replay: FnOnce(ClockTrace) -> Option<Result<u64, TraceError>>,
    {
        let mode = self.mode();

        match mode {
            // fast and strict modes execute directly
            ExecutionMode::Fast | ExecutionMode::Strict => call(),
            // replay mode reads one recorded clock fact
            ExecutionMode::Replay => {
                on_replay_read();
                let trace = self.next_clock_trace()?;
                if trace.subject() != Some(subject) {
                    return Err(self.entropy_mismatch_error());
                }

                match replay(trace) {
                    Some(outcome) => outcome.map_err(Box::<RuntimeError>::from),
                    None => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one clock fact
            ExecutionMode::Record => {
                let result = call();
                let outcome = result
                    .as_ref()
                    .map(|value| *value)
                    .map_err(|error| TraceError::from(error.as_ref()));
                let trace = record(subject, outcome);
                self.record(Trace::Clock(trace))?;

                result
            }
        }
    }
}
