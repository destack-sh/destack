use crate::{Node, NodeType, StringId};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyMode {
    /// Export as regular item (like `export foo`).
    Item,
    /// Export as default item (like `export default foo`).
    Default,
    /// Export as entire namespace (like `export = foo`).
    Namespace,
}

/// The kind of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyKind {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// A DependencyItem is an item to import from a target in an import or export clause.
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
    const TYPE: NodeType = NodeType::DependencyItem;
}
