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

/// A ImportTarget is the target to import from.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportTarget {
    // Regular virtual target as an identifier/path (like `foo` or `foo.bar`)
    Virtual(Path),
    // String target as a literal string (like `"foo"` or `"foo/bar"`)
    Physical(StringId),
}

/// A ImportItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    /// The target of the item.
    pub target: ImportTarget,
    /// The name to import.
    pub name: StringId,
    /// The alias to use for the item.
    pub alias: Option<StringId>,
}

impl Node for ImportItem {
    const KIND: NodeType = NodeType::ImportItem;
}
