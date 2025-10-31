use dyst_source::StringId;

use crate::{Node, NodeType, Path};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportType {
    // Export as regular item (export foo)
    Item,
    // Export as default item (export default foo)
    Default,
}

/// A DependencyTarget is the target to import from.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyTarget {
    // Regular Path target as an identifier/path (like `foo` or `foo.bar`)
    Path(Path),
    // Module string target as a literal string (like `"foo"` or `"foo/bar"`)
    String(StringId),
}

/// The type of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyKind {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// A DependencyItem is an item to import from a target in a import clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DependencyItem {
    /// The type of the item (if specified).
    pub kind: Option<DependencyKind>,
    /// The source of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for DependencyItem {
    const KIND: NodeType = NodeType::DependencyItem;
}
