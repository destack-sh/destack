use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use std::num::NonZeroU32;

use destack_core::StringId;

/// How one derived tree node came to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// The transform that created the node, a dotted name like `optimize.inline`.
    pub derivation: StringId,
    /// The same-tree nodes the node derives from, interpretation per derivation.
    pub parents: SmallVec<[u32; 2]>,
}

impl Origin {
    /// Create one origin with explicit parents.
    pub fn new(derivation: StringId, parents: impl IntoIterator<Item = u32>) -> Self {
        Self {
            derivation,
            parents: parents.into_iter().collect(),
        }
    }

    /// Create one origin with a single parent.
    pub fn one(derivation: StringId, parent: u32) -> Self {
        Self::new(derivation, [parent])
    }

    /// Create one origin with no parents.
    pub fn synthetic(derivation: StringId) -> Self {
        Self::new(derivation, [])
    }

    /// Return the primary parent node.
    pub fn parent(&self) -> Option<u32> {
        self.parents.first().copied()
    }
}

/// Origin records for derived nodes: dense per-node slots into a packed arena.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OriginTable {
    /// The arena slot for each node index, present for derived nodes.
    slot_by_index: Vec<Option<NonZeroU32>>,
    /// The packed origin records, emptied when an origin moves to another node.
    origins: Vec<Option<Origin>>,
}

impl OriginTable {
    /// Append one empty slot for a new node.
    #[inline]
    pub fn append(&mut self) {
        self.slot_by_index.push(None);
    }

    /// Set the origin for one node index.
    pub fn set(&mut self, index: usize, origin: Origin) {
        // reuse the existing arena record when present
        if let Some(slot) = self.slot_by_index[index] {
            self.origins[slot.get() as usize - 1] = Some(origin);
            return;
        }

        self.origins.push(Some(origin));
        let slot = NonZeroU32::new(self.origins.len() as u32)
            .unwrap_or_else(|| unreachable!("origin arena slot overflowed"));
        self.slot_by_index[index] = Some(slot);
    }

    /// Return the origin for one node index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Origin> {
        let slot = self.slot_by_index.get(index).copied().flatten()?;

        self.origins[slot.get() as usize - 1].as_ref()
    }

    /// Move the origin off one node index, leaving the slot empty.
    pub fn take(&mut self, index: usize) -> Option<Origin> {
        let slot = self.slot_by_index[index].take()?;

        self.origins[slot.get() as usize - 1].take()
    }
}
