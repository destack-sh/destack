use destack_core::StringPool;
use serde::{Deserialize, Serialize};

use crate::{LocalNodeIdAny, Tree};

/// One lowered JavaScript or TypeScript module tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// The lowered script tree.
    pub tree: Tree,
    /// The root nodes in the lowered tree.
    pub roots: Vec<LocalNodeIdAny>,
    /// The string pool for the lowered tree.
    pub strings: StringPool,
}
