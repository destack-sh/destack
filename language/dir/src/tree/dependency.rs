use dyst_source::StringId;

use crate::{Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, ModuleId, Node, NodeType};

/// The source of a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub enum DependencySource {
    /// Plain import statement (like `import "foo"`).
    ImportStatement,
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
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub enum DependencyMode {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`)
    Item,
    /// Default item (`export default foo`)
    Default,
    /// Namespace (`export * from "foo"` or `export = foo`)
    Namespace,
}

/// The type of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub enum DependencyKind {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// A DependencyItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyItem {
    /// Unresolved remote item aliased to a local item from a target.
    UnresolvedRemote {
        source: DependencySource,
        mode: DependencyMode,
        kind: DependencyKind,
        name: Option<StringId>,
        alias: Option<StringId>,
        target: StringId,
        target_module: Option<ModuleId>, // item may remain unresolved even if we can resolve the target module
        symbol: LocalSymbolId,
    },
    /// Unresolved local item from the module.
    UnresolvedLocal {
        mode: DependencyMode,
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        symbol: LocalSymbolId,
    },
    /// Value expression dependency (like `export = foo`).
    Value { value: LocalNodeId<Expression> },
    /// Internal to the module (i.e., plain exports).
    Local {
        mode: DependencyMode,
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        symbol: LocalSymbolId,
        target_symbol: LocalSymbolId,
    },
    /// Remote to the module (i.e., imports and re-exports).
    Remote {
        mode: DependencyMode,
        kind: DependencyKind,
        name: Option<StringId>,
        alias: Option<StringId>,
        target: StringId,
        target_module: ModuleId,
        symbol: LocalSymbolId,
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

impl DependencyItem {
    /// Get the symbol of the dependency item.
    pub fn symbol(&self) -> Option<LocalSymbolId> {
        match self {
            DependencyItem::UnresolvedRemote { symbol, .. } => Some(*symbol),
            DependencyItem::UnresolvedLocal { symbol, .. } => Some(*symbol),
            DependencyItem::Value { .. } => None,
            DependencyItem::Local { symbol, .. } => Some(*symbol),
            DependencyItem::Remote { symbol, .. } => Some(*symbol),
        }
    }
}
