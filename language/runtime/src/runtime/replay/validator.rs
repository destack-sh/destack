use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{EntropyEvent, ReplayEvent};

/// Validation state for replay event ordering.
#[derive(Debug, Default)]
pub(super) struct ReplayValidator {
    /// Last observed task queue sequence.
    last_task_queue_sequence: Option<u64>,
    /// Last observed monotonic time sample.
    last_monotonic_nanos: Option<u64>,
}

impl ReplayValidator {
    /// Validate a replay event against ordering invariants.
    pub(super) fn validate(&mut self, event: &ReplayEvent) -> RuntimeResult<()> {
        match event {
            ReplayEvent::TaskQueueEvent(event) => {
                if let Some(last) = self.last_task_queue_sequence
                    && event.sequence < last
                {
                    return Err(RuntimeError::ReplayMismatch {
                        name: "event_loop".to_string(),
                    }
                    .boxed());
                }
                self.last_task_queue_sequence = Some(event.sequence);
            }
            ReplayEvent::Entropy(event) => match event {
                EntropyEvent::TimeReadMonotonic { outcome, .. } => {
                    if let Ok(time_nanos) = outcome {
                        if let Some(last) = self.last_monotonic_nanos
                            && *time_nanos < last
                        {
                            return Err(RuntimeError::ReplayMismatch {
                                name: "entropy".to_string(),
                            }
                            .boxed());
                        }
                        self.last_monotonic_nanos = Some(*time_nanos);
                    }
                }
                EntropyEvent::RandomReadBytes {
                    len,
                    outcome: Ok(bytes),
                    ..
                } => {
                    if bytes.len() != *len as usize {
                        return Err(RuntimeError::ReplayMismatch {
                            name: "entropy".to_string(),
                        }
                        .boxed());
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }
}
