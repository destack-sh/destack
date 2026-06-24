use destack_core::StringPool;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{LocalNodeIdAny, Tree};

/// One lowered JavaScript or TypeScript module tree.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Module {
    /// The lowered script tree.
    pub tree: Tree,
    /// The root nodes in the lowered tree.
    pub roots: Vec<LocalNodeIdAny>,
    /// The string pool for the lowered tree.
    pub strings: StringPool,
}
