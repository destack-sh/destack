use serde::{Deserialize, Serialize};

/// Runtime check configuration for a VM isolate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckOptions {
    /// Whether field and element access should check bounds.
    pub bounds: bool,
    /// Whether pointer dereferences should check null pointers.
    pub null: bool,
    /// Whether reference kind constraints should be checked.
    pub reference_kind: bool,
    /// Whether reference mutability rules should be checked.
    pub reference_mutability: bool,
}

impl CheckOptions {
    /// Create strict runtime checks for debug execution.
    pub fn debug() -> Self {
        Self {
            bounds: true,
            null: true,
            reference_kind: true,
            reference_mutability: true,
        }
    }
}
