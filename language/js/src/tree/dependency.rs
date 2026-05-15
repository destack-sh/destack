use crate::{Expression, LocalNodeId, Name, Node, NodeType, StringId};

use serde::{Deserialize, Serialize};

/// How one dependency item binds into the local module or export surface.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyBinding {
    /// Named binding (`import { foo } from "foo"` or `export { foo } from "foo"`).
    Named,
    /// Default binding (`export default foo`).
    Default,
    /// Namespace binding (`import * as foo from "foo"` or `export * from "foo"`).
    Namespace,
}

/// The source form of one dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyForm {
    /// Type-marked dependency (`import type foo` or `export type foo`).
    Type,
    /// Plain dependency (`import foo` or `export foo`).
    Plain,
}

/// One dependency binding in an import or export clause.
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
    /// The source form of the item, when specified.
    pub form: Option<DependencyForm>,
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
