use crate::{Expression, LocalNodeId, Name, Node, NodeType, StringId};

use serde::{Deserialize, Serialize};
/// How one dependency item binds into the local module or export surface.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyBinding {
    /// Export as regular item (like `export foo`).
    Item,
    /// Export as default item (like `export default foo`).
    Default,
    /// Export as entire namespace (like `export = foo`).
    Namespace,
}

/// The space of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencySpace {
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
    /// How the item binds.
    pub binding: DependencyBinding,
    /// The symbol space of the item, when specified.
    pub space: Option<DependencySpace>,
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
