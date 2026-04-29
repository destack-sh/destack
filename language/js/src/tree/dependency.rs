use crate::{Expression, LocalNodeId, Name, Node, NodeType, StringId};

use serde::{Deserialize, Serialize};
/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyMode {
    /// Export as regular item (like `export foo`).
    Item,
    /// Export as default item (like `export default foo`).
    Default,
    /// Export as entire namespace (like `export = foo`).
    Namespace,
}

/// The kind of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependencyItem {
    /// The mode of the item (Item, Default, Namespace).
    pub mode: DependencyMode,
    /// The type of the item (if specified, like `type` in `import type foo`).
    pub kind: Option<DependencyKind>,
    /// The name of the item (like `foo` in `foo as bar`).
    /// None for default/namespace items where only alias matters.
    pub name: Option<Name>,
    /// The alias to use for the item (like `bar` in `foo as bar`).
    pub alias: Option<StringId>,
    /// The value of the item (for `export = foo` style exports).
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}
