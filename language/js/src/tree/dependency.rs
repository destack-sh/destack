use crate::{Name, Node, NodeType, StringId};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// How one dependency item binds into the local module or its exports.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum DependencyBinding {
    /// Named binding (`import { foo } from "foo"` or `export { foo } from "foo"`).
    Named,
    /// Default binding (`export default foo`).
    Default,
    /// Namespace binding (`import * as foo from "foo"` or `export * from "foo"`).
    Namespace,
}

/// How one declaration is exported.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ExportKind {
    /// Named export.
    Named,
    /// Default export.
    Default,
}

/// One dependency binding in an import or export clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DependencyItem {
    /// How the item binds.
    pub binding: DependencyBinding,
    /// The name of the item (like `foo` in `foo as bar`).
    /// None for default/namespace items where only alias matters.
    pub name: Option<Name>,
    /// The alias to use for the item (like `bar` in `foo as bar`).
    pub alias: Option<StringId>,
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}
