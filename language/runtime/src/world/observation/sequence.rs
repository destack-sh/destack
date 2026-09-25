use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Stable sequence number for one observation entry.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ObservationSequence(u64);

impl ObservationSequence {
    /// Create one observation sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw observation sequence.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next observation sequence.
    pub fn next(self) -> RuntimeResult<Self> {
        let value = self.0.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "observation sequence space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(Self(value))
    }
}
