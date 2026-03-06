use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{EntropyEvent, ReplayEvent};

/// Validation state for replay event ordering.
#[derive(Debug, Default)]
pub(super) struct ReplayValidator {
    /// Last observed monotonic time sample.
    last_monotonic_nanos: Option<u64>,
}

impl ReplayValidator {
    /// Validate a replay event against ordering invariants.
    pub(super) fn validate(&mut self, event: &ReplayEvent) -> RuntimeResult<()> {
        match event {
            ReplayEvent::Tick(deadline) => {
                if let Some(last) = self.last_monotonic_nanos
                    && deadline.get() < last
                {
                    return Err(RuntimeError::ReplayMismatch {
                        name: "tick".to_string(),
                    }
                    .boxed());
                }

                self.last_monotonic_nanos = Some(deadline.get());
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
