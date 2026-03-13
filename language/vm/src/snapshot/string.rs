use std::collections::BTreeMap;

use destack_heap::{ManagedReference, RawPointer};
use serde::{Deserialize, Serialize};

/// Durable string interner state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringInternerImage {
    /// Interned string literals keyed by string contents.
    pub literals: BTreeMap<String, ManagedReference>,
    /// Raw heap buffers keyed by managed string handles.
    pub buffers: BTreeMap<ManagedReference, RawPointer>,
}
