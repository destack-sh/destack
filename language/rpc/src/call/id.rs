use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Caller-scoped RPC call identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CallId(pub u64);

impl CallId {
    /// Create one call identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}
