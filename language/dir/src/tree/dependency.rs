use dyst_source::StringId;

use crate::{Expression, GlobalSymbolId, LocalNodeId, LocalSymbolId, ModuleId, Node, NodeType};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyMode {
    /// Export as regular item (export foo)
    Item,
    /// Export as default item (export default foo)
    Default,
    /// Export as entire namespace (export * from foo)
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
    /// Unresolved default from a target (like `import * as foo from "foo"`).
    UnresolvedRemoteDefault {
        kind: DependencyKind,
        alias: StringId,
        target: StringId,
        symbol: LocalSymbolId,
    },
    /// Import or export a single item from a target (`import { foo } from "foo"` or `export { foo } from "foo"`).
    UnresolvedRemoteItem {
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        target: StringId,
        symbol: LocalSymbolId,
    },
    /// Export a default item from the module (like `export default foo`).
    UnresolvedLocalDefault {
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
    },
    /// Export a single item from the module (like `export { foo }`).
    UnresolvedLocalItem {
        kind: DependencyKind,
        name: StringId,
    },
    /// Value expression dependency (like `export = foo`).
    Value { value: LocalNodeId<Expression> },
    /// Internal to the module (i.e., plain exports).
    Local {
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        symbol: LocalSymbolId,
    },
    /// Remote to the module (i.e., imports and re-exports).
    Remote {
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        target: StringId,
        symbol: LocalSymbolId,
        target_symbol: GlobalSymbolId,
        module: ModuleId,
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
