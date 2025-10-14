use dyst_source::StringId;

use crate::{Node, NodeType, Path};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportMode {
    // Export as regular item (export foo)
    Item,
    // Export as default item (export default foo)
    Default,
}

/// A ImportItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    /// The source of the item.
    pub source: Path,
    /// The alias to use for the item.
    pub alias: Option<StringId>,
}

impl Node for ImportItem {
    const KIND: NodeType = NodeType::ImportItem;
}
