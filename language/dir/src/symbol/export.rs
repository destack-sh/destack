use crate::{DependencyItem, LocalNodeId, LocalSymbolId};

/// An Export is a resolved module export entry.
#[derive(Debug, Clone, PartialEq)]
pub enum Export {
    /// Local symbol export.
    Local { symbol: LocalSymbolId },
    /// Re-export via a dependency item.
    ReExport { item: LocalNodeId<DependencyItem> },
}
