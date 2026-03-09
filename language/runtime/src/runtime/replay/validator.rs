use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{EntropyEvent, TraceEvent};
use serde::{Deserialize, Serialize};

/// Validation state for trace event ordering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TraceValidator {
    /// Last observed monotonic time sample.
    last_monotonic_nanos: Option<u64>,
}

impl TraceValidator {
    /// Validate a trace event against ordering invariants.
    pub(super) fn validate(&mut self, event: &TraceEvent) -> RuntimeResult<()> {
        match event {
            TraceEvent::Tick(deadline) => {
                if let Some(last) = self.last_monotonic_nanos
                    && deadline.get() < last
                {
                    return Err(RuntimeError::TraceMismatch {
                        name: "tick".to_string(),
                    }
                    .boxed());
                }

                self.last_monotonic_nanos = Some(deadline.get());
            }
            TraceEvent::Entropy(event) => match event {
                EntropyEvent::TimeReadMonotonic { outcome, .. } => {
                    if let Ok(time_nanos) = outcome {
                        if let Some(last) = self.last_monotonic_nanos
                            && *time_nanos < last
                        {
                            return Err(RuntimeError::TraceMismatch {
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
                        return Err(RuntimeError::TraceMismatch {
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
