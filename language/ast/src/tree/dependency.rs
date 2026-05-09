use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Name, Node, NodeType};

/// How one dependency item binds into the local module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyBinding {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`)
    Item,
    /// Default item (`export default foo`)
    Default,
    /// Namespace (`export * from "foo"`).
    Namespace,
}

/// The export kind of a declaration or binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExportKind {
    /// Named export (`export const foo = 1`).
    Named,
    /// Default export (`export default foo`).
    Default,
}

/// The symbol space one dependency item imports or exports.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencySpace {
    /// Type dependency (`import type { Foo }` or `export type { Foo }`).
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
        /// How the item binds into the local module.
        binding: DependencyBinding,
        /// The symbol space of the item, when specified.
        space: Option<DependencySpace>,
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
