use dyst_source::StringId;

use crate::{Node, NodeType, Path};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportMode {
    // Export as regular item (export foo)
    Item,
    // Export as default item (export default foo)
    Default,
}

/// A ImportTarget is the target to import from.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportTarget {
    // Regular Path target as an identifier/path (like `foo` or `foo.bar`)
    Path(Path),
    // Virtual string target as a literal string (like `"foo"` or `"foo/bar"`)
    Virtual(StringId),
}

/// A ImportItem is an item to import from a target in a import clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    /// The source of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for ImportItem {
    const KIND: NodeType = NodeType::ImportItem;
}
