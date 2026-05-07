use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, LocalSymbolId, ModuleTarget, Name, Node, NodeType};

/// How one dependency item binds into the local module.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum DependencyBinding {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`)
    Item,
    /// Default item (`export default foo`)
    Default,
    /// Namespace (`export * from "foo"` or `export = foo`)
    Namespace,
}

/// The export kind of a declaration or binding.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum ExportKind {
    /// Named export (`export const foo = 1`).
    Named,
    /// Default export (`export default foo`).
    Default,
}

/// The symbol space one dependency item imports or exports.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum DependencySpace {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// One dependency item imported from or exported to another module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencyItem {
    /// Malformed dependency item slot.
    Error,
    /// Named imported or exported item.
    Item {
        binding: DependencyBinding,
        space: DependencySpace,
        name: Option<Name>,
        alias: Option<StringId>,
        symbol: Option<LocalSymbolId>,
    },
    /// Value expression dependency (like `export = foo` or `export default foo`).
    Value {
        binding: DependencyBinding,
        value: LocalNodeId<Expression>,
    },
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}

/// A namespace export edge from `export * from` declarations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NamespaceExport {
    /// The target module.
    pub module_id: ModuleTarget,
    /// The dependency space for the export.
    pub space: DependencySpace,
    /// The dependency item node that declared the export.
    pub item: LocalNodeId<DependencyItem>,
}

impl DependencyItem {
    /// Get the symbol of the dependency item.
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            DependencyItem::Error => None,
            DependencyItem::Value { .. } => None,
            DependencyItem::Item { symbol, .. } => *symbol,
        }
    }
}
