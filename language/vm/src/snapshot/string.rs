use std::collections::BTreeMap;
use std::mem::size_of;

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

impl StringInternerImage {
    /// Return the owned bytes for this string-interner image.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += btree_map_bytes::<String, ManagedReference>(self.literals.len());
        owned_bytes += btree_map_bytes::<ManagedReference, RawPointer>(self.buffers.len());

        for literal in self.literals.keys() {
            owned_bytes += literal.capacity();
        }

        owned_bytes
    }
}

/// Approximate per-entry overhead for one btree map entry.
const BTREE_ENTRY_OVERHEAD_BYTES: usize = size_of::<usize>() * 3;

/// Return the approximate owned bytes for one btree map table.
fn btree_map_bytes<K, V>(len: usize) -> usize {
    size_of::<BTreeMap<K, V>>()
        + len * (size_of::<K>() + size_of::<V>() + BTREE_ENTRY_OVERHEAD_BYTES)
}
