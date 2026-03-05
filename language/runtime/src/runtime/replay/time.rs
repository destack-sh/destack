use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{ReplayController, ReplayEvent, TimeEvent, TimeEventKind};
use destack_workspace::ExecutionMode;

/// Replay channel name for time events.
const TIME_CHANNEL: &str = "time";

/// Return one replay mismatch error for the time channel.
fn time_mismatch_error() -> Box<RuntimeError> {
    RuntimeError::ReplayMismatch {
        name: TIME_CHANNEL.to_string(),
    }
    .boxed()
}

impl ReplayController {
    /// Read the next time event for replay.
    pub fn next_time_event(&self, expected: TimeEventKind) -> RuntimeResult<TimeEvent> {
        // reject reads outside replay execution
        if self.mode() != ExecutionMode::Replay {
            return Err(time_mismatch_error());
        }

        // read the next event from the log
        let Some(event) = self.next_event()? else {
            let sequence = self.log().next_sequence().get();
            return Err(RuntimeError::ReplayLogExhausted { sequence }.boxed());
        };

        // validate the time event
        let ReplayEvent::TimeEvent(time_event) = event else {
            return Err(time_mismatch_error());
        };

        // validate the time event kind
        if time_event.kind != expected {
            return Err(time_mismatch_error());
        }

        Ok(time_event)
    }

    /// Run a time binding that reads a timestamp.
    pub fn run_time_read<Call>(&self, kind: TimeEventKind, call: Call) -> RuntimeResult<u64>
    where
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        let mode = self.mode();

        match mode {
            // fast and deterministic modes execute directly
            ExecutionMode::Fast | ExecutionMode::Deterministic => call(),
            // replay mode reads one recorded time sample
            ExecutionMode::Replay => {
                let event = self.next_time_event(kind)?;
                Ok(event.time_nanos)
            }
            // record mode executes and records one time sample
            ExecutionMode::Record => {
                let value = call()?;
                self.record_event(ReplayEvent::TimeEvent(TimeEvent {
                    kind,
                    time_nanos: value,
                    interval_nanos: None,
                    timer_id: None,
                }))?;

                Ok(value)
            }
        }
    }
}
