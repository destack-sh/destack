use destack_mir::{self as mir};
use serde::{Deserialize, Serialize};

/// Lowered MIR payload before optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirLowered {
    /// The MIR tree.
    pub tree: mir::Tree,
}

impl MirLowered {
    /// Create a new lowered MIR payload.
    pub fn new() -> Self {
        Self {
            tree: mir::Tree::new(),
        }
    }
}

impl Default for MirLowered {
    fn default() -> Self {
        Self::new()
    }
}

/// Verified MIR patch after required semantic verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirVerified {
    /// Required verification patch.
    pub patch: mir::Patch,
}

impl MirVerified {
    /// Create an empty verified MIR payload over one lowered tree.
    pub fn new(base: &mir::Tree) -> Self {
        Self {
            patch: mir::Patch::new(base, "verify"),
        }
    }
}

/// Optimized MIR payload after pipeline transforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirOptimized {
    /// Ordered optimization patches.
    pub patches: Vec<mir::Patch>,
}

impl MirOptimized {
    /// Create a new optimized MIR payload.
    pub fn new() -> Self {
        Self {
            patches: Vec::new(),
        }
    }

    /// Return the tree owned by the last optimization patch.
    pub fn latest_patch_tree(&self) -> Option<&mir::Tree> {
        self.patches.last().map(|patch| &patch.tree)
    }
}

impl Default for MirOptimized {
    fn default() -> Self {
        Self::new()
    }
}
