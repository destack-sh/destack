use serde::{Deserialize, Serialize};
use tspp_core::StringPool;
use tspp_serde::Reflect;

use crate::{LocalNodeIdAny, Tree};

/// One lowered JavaScript module tree.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Module {
    /// The lowered script tree.
    pub tree: Tree,
    /// The root nodes in the lowered tree.
    pub roots: Vec<LocalNodeIdAny>,
    /// The string pool for the lowered tree.
    pub strings: StringPool,
}
