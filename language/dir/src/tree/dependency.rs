use destack_core::StringId;
use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{
    Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, ModuleResolution, ModuleTarget, Name,
    Node, NodeType,
};

/// The source of a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize, AdaptImage)]
pub enum DependencySource {
    /// Plain import statement (like `import "foo"`).
    ImportStatement,
    /// TypeScript triple-slash `reference path` directive.
    ReferencePathDirective,
    /// TypeScript triple-slash `reference types` directive.
    ReferenceTypesDirective,
    /// TypeScript triple-slash `reference lib` directive.
    ReferenceLibDirective,
    /// Import-equals statement (like `import foo = require("foo")`).
    ImportEquals,
    /// Re-export statement (like `export { bar } from "foo"`).
    ExportStatement,
    /// Import call (like `await import("foo")`).
    ImportCall,
    /// Require call (like `require("foo")`).
    RequireCall,
    /// Value expression (like `export = foo`).
    ValueExpression,
}

/// The mode of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize, AdaptImage)]
pub enum DependencyMode {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`)
    Item,
    /// Default item (`export default foo`)
    Default,
    /// Namespace (`export * from "foo"` or `export = foo`)
    Namespace,
}

/// The export mode of a declaration or binding.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize, AdaptImage)]
pub enum ExportMode {
    /// Named export (`export const foo = 1`).
    Named,
    /// Default export (`export default foo`).
    Default,
}

/// The type of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize, AdaptImage)]
pub enum DependencyKind {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// A DependencyItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum DependencyItem {
    /// Malformed dependency item slot.
    Error,
    /// Unresolved remote item aliased to a local item from a target.
    UnresolvedRemote {
        source: DependencySource,
        mode: DependencyMode,
        kind: DependencyKind,
        name: Option<Name>,
        alias: Option<StringId>,
        target: StringId,
        target_module: Option<ModuleResolution>, // item may remain unresolved even if we can resolve the target module
        symbol: Option<LocalSymbolId>,
    },
    /// Unresolved local item from the module.
    UnresolvedLocal {
        mode: DependencyMode,
        kind: DependencyKind,
        name: Option<Name>,
        alias: Option<StringId>,
        symbol: Option<LocalSymbolId>,
    },
    /// Value expression dependency (like `export = foo` or `export default foo`).
    Value {
        mode: DependencyMode,
        value: LocalNodeId<Expression>,
    },
    /// Internal to the module (i.e., plain exports).
    Local {
        mode: DependencyMode,
        kind: DependencyKind,
        name: Option<Name>,
        alias: Option<StringId>,
        symbol: Option<LocalSymbolId>,
        target_symbol: GlobalSymbolId,
    },
    /// Remote to the module (i.e., imports and re-exports).
    Remote {
        mode: DependencyMode,
        kind: DependencyKind,
        name: Option<Name>,
        alias: Option<StringId>,
        target: StringId,
        target_module: ModuleResolution,
        symbol: Option<LocalSymbolId>,
        target_symbol: GlobalSymbolId,
    },
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;

    fn is_resolved(&self) -> bool {
        matches!(
            self,
            DependencyItem::Local { .. }
                | DependencyItem::Remote { .. }
                | DependencyItem::Value { .. }
        )
    }
}

/// A namespace export edge from `export * from` declarations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct NamespaceExport {
    /// The target module.
    pub module_id: ModuleTarget,
    /// The dependency kind for the export.
    pub kind: DependencyKind,
    /// The dependency item node that declared the export.
    pub item: LocalNodeId<DependencyItem>,
}

impl DependencyItem {
    /// Get the symbol of the dependency item.
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            DependencyItem::Error => None,
            DependencyItem::UnresolvedRemote { symbol, .. } => *symbol,
            DependencyItem::UnresolvedLocal { symbol, .. } => *symbol,
            DependencyItem::Value { .. } => None,
            DependencyItem::Local { symbol, .. } => *symbol,
            DependencyItem::Remote { symbol, .. } => *symbol,
        }
    }

    /// Get the target symbol of the dependency item.
    pub fn target_symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            DependencyItem::Error => None,
            DependencyItem::UnresolvedRemote { .. } => None,
            DependencyItem::UnresolvedLocal { .. } => None,
            DependencyItem::Value { .. } => None,
            DependencyItem::Local { target_symbol, .. } => Some(*target_symbol),
            DependencyItem::Remote { target_symbol, .. } => Some(*target_symbol),
        }
    }

    /// Get the resolved target module for a dependency kind.
    pub fn target_module_for_kind(&self, kind: DependencyKind) -> Option<ModuleTarget> {
        match self {
            DependencyItem::Error => None,
            DependencyItem::Remote { target_module, .. } => target_module.for_kind(kind),
            DependencyItem::UnresolvedRemote { target_module, .. } => {
                target_module.and_then(|targets| targets.for_kind(kind))
            }
            _ => None,
        }
    }
}
