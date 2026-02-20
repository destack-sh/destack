use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::ExecutionMode;
use crate::runtime::replay::{ReplayController, ReplayEvent, TimeEvent, TimeEventKind};

impl ReplayController {
    /// Read the next time event for replay.
    pub fn next_time_event(&self, expected: TimeEventKind) -> RuntimeResult<TimeEvent> {
        // reject reads outside replay execution
        if self.mode() != ExecutionMode::Replay {
            return Err(RuntimeError::ReplayMismatch {
                name: "time".to_string(),
            }
            .boxed());
        }

        // read the next event from the log
        let Some(event) = self.next_event()? else {
            let sequence = self.log().next_sequence().get();
            return Err(RuntimeError::ReplayLogExhausted { sequence }.boxed());
        };

        // validate the time event
        let ReplayEvent::TimeEvent(time_event) = event else {
            return Err(RuntimeError::ReplayMismatch {
                name: "time".to_string(),
            }
            .boxed());
        };

        // validate the time event kind
        if time_event.kind != expected {
            return Err(RuntimeError::ReplayMismatch {
                name: "time".to_string(),
            }
            .boxed());
        }

        Ok(time_event)
    }

    /// Run a time binding that reads a timestamp.
    pub fn run_time_read<Call>(&self, kind: TimeEventKind, call: Call) -> RuntimeResult<u64>
    where
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        // fast path
        if !cfg!(feature = "replay") || self.mode() == ExecutionMode::Fast {
            return call();
        }

        // replay path
        if self.mode() == ExecutionMode::Replay {
            let event = self.next_time_event(kind)?;
            return Ok(event.time_nanos);
        }

        // record path
        let value = call()?;
        if self.mode() == ExecutionMode::Record {
            self.record_event(ReplayEvent::TimeEvent(TimeEvent {
                kind,
                time_nanos: value,
                interval_nanos: None,
                timer_id: None,
            }))?;
        }

        Ok(value)
    }
}
