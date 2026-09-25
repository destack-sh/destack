use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_serde::Reflect;

use std::num::NonZeroU32;

use tspp_core::StringId;

/// How one derived tree node came to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Provenance {
    /// The transform that created the node, a dotted name like `optimize.inline`.
    pub derivation: StringId,
    /// The same-tree nodes the node derives from, interpretation per derivation.
    pub parents: SmallVec<[u32; 2]>,
}

impl Provenance {
    /// Create one provenance with explicit parents.
    pub fn new(derivation: StringId, parents: impl IntoIterator<Item = u32>) -> Self {
        Self {
            derivation,
            parents: parents.into_iter().collect(),
        }
    }

    /// Create one provenance with a single parent.
    pub fn one(derivation: StringId, parent: u32) -> Self {
        Self::new(derivation, [parent])
    }

    /// Create one provenance with no parents.
    pub fn synthetic(derivation: StringId) -> Self {
        Self::new(derivation, [])
    }

    /// Return the primary parent node.
    pub fn parent(&self) -> Option<u32> {
        self.parents.first().copied()
    }
}

/// Provenance records for derived nodes: dense per-node slots into a packed arena.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ProvenanceTable {
    /// The arena slot for each node index, present for derived nodes.
    slot_by_index: Vec<Option<NonZeroU32>>,
    /// The packed provenance records, emptied when an provenance moves to another node.
    provenances: Vec<Option<Provenance>>,
}

impl ProvenanceTable {
    /// Append one empty slot for a new node.
    #[inline]
    pub fn append(&mut self) {
        self.slot_by_index.push(None);
    }

    /// Set the provenance for one node index.
    pub fn set(&mut self, index: usize, provenance: Provenance) {
        // reuse the existing arena record when present
        if let Some(slot) = self.slot_by_index[index] {
            self.provenances[slot.get() as usize - 1] = Some(provenance);
            return;
        }

        self.provenances.push(Some(provenance));
        let slot = NonZeroU32::new(self.provenances.len() as u32)
            .unwrap_or_else(|| unreachable!("provenance arena slot overflowed"));
        self.slot_by_index[index] = Some(slot);
    }

    /// Return the provenance for one node index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Provenance> {
        let slot = self.slot_by_index.get(index).copied().flatten()?;

        self.provenances[slot.get() as usize - 1].as_ref()
    }

    /// Move the provenance off one node index, leaving the slot empty.
    pub fn take(&mut self, index: usize) -> Option<Provenance> {
        let slot = self.slot_by_index[index].take()?;

        self.provenances[slot.get() as usize - 1].take()
    }
}
