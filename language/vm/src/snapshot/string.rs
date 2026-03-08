use std::collections::BTreeMap;

use destack_heap::{ManagedPointer, RawPointer};
use serde::{Deserialize, Serialize};

/// Durable string interner state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringInternerSnapshot {
    /// Interned string literals keyed by string contents.
    pub literals: BTreeMap<String, ManagedPointer>,
    /// Raw heap buffers keyed by managed string handles.
    pub buffers: BTreeMap<ManagedPointer, RawPointer>,
}
