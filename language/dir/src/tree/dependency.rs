use dyst_source::StringId;

use crate::{Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, ModuleId, Node, NodeType};

/// The mode of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyMode {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`)
    Item,
    /// Default item (`export default foo`)
    Default,
    /// Namespace (`export * from "foo"` or `export = foo`)
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

/// A DependencyItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyItem {
    /// Unresolved remote item aliased to a local item from a target.
    UnresolvedRemote {
        mode: DependencyMode,
        kind: DependencyKind,
        alias: Option<StringId>,
        target: StringId,
        symbol: LocalSymbolId,
    },
    /// Unresolved local item from the module.
    UnresolvedLocal {
        mode: DependencyMode,
        kind: DependencyKind,
        name: StringId,
    },
    /// Value expression dependency (like `export = foo`).
    Value { value: LocalNodeId<Expression> },
    /// Internal to the module (i.e., plain exports).
    Local {
        mode: DependencyMode,
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        target_symbol: LocalSymbolId,
    },
    /// Remote to the module (i.e., imports and re-exports).
    Remote {
        mode: DependencyMode,
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        target: StringId,
        module: ModuleId,
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
