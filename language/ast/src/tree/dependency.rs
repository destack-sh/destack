use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Name, Node, NodeType};

/// The mode of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyMode {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`)
    Item,
    /// Default item (`export default foo`)
    Default,
    /// Namespace (`export * from "foo"` or `export = foo`)
    Namespace,
}

/// The export mode of a declaration or binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExportMode {
    /// Named export (`export const foo = 1`).
    Named,
    /// Default export (`export default foo`).
    Default,
}

/// The type of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyKind {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// A DependencyItem is an item to import / export from a target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencyItem {
    /// One valid dependency item.
    ///
    /// Examples:
    /// ```
    /// baz
    /// qux as quux
    /// default
    /// default as bar
    /// ```
    Item {
        /// The type of the item.
        mode: DependencyMode,
        /// The type of the item (if specified).
        kind: Option<DependencyKind>,
        /// The name of the item (like `foo` in `foo as bar`, None if default).
        name: Option<Name>,
        /// The alias to use for the item (like `bar` in `foo as bar`)
        alias: Option<StringId>,
        /// The value of the item (for namespace exports)
        value: Option<LocalNodeId<Expression>>,
    },
    /// One malformed dependency item slot.
    Error,
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}
