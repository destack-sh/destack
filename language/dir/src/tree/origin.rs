use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

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
