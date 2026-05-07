use destack_core::StringPool;
use destack_mir::{self as mir};
use serde::{Deserialize, Serialize};

/// Lowered MIR payload before optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirLowered {
    /// The MIR tree.
    pub tree: mir::Tree,
    /// The MIR string pool.
    pub strings: StringPool,
}

impl MirLowered {
    /// Create a new lowered MIR payload.
    pub fn new() -> Self {
        Self {
            tree: mir::Tree::new(),
            strings: StringPool::new(),
        }
    }
}

impl Default for MirLowered {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimized MIR payload after pipeline transforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirOptimized {
    /// The optimized MIR tree.
    pub tree: mir::Tree,
    /// The MIR string pool.
    pub strings: StringPool,
}

impl MirOptimized {
    /// Create a new optimized MIR payload.
    pub fn new() -> Self {
        Self {
            tree: mir::Tree::new(),
            strings: StringPool::new(),
        }
    }
}

impl Default for MirOptimized {
    fn default() -> Self {
        Self::new()
    }
}
