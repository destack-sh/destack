use dyst_source::StringId;

use crate::{Expression, LocalNodeId, Node, NodeType};

/// The mode of an export.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyMode {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`)
    Item,
    /// Default item (`export default foo`)
    Default,
    /// Namespace (`export = foo`)
    Namespace,
}

/// The type of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyKind {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// A DependencyItem is an item to import / export from a target.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// default
/// default as bar
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DependencyItem {
    /// The type of the item.
    pub mode: DependencyMode,
    /// The type of the item (if specified).
    pub kind: Option<DependencyKind>,
    /// The name of the item (like `foo` in `foo as bar`, None if default)
    pub name: Option<StringId>,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
    /// The value of the item (for namespace exports)
    pub value: Option<LocalNodeId<Expression>>,
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}
