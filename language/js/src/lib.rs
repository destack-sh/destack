#![feature(default_field_values)]

mod tree;
mod walk;

pub use tree::*;
pub use walk::*;

/// One generated script module.
#[derive(Debug, Clone)]
pub struct ScriptModule {
    /// The lowered script tree.
    pub tree: NodeTree,
    /// The root nodes in the lowered tree.
    pub roots: Vec<LocalNodeIdAny>,
    /// The string pool for the lowered tree.
    pub strings: destack_core::StringPool,
}
