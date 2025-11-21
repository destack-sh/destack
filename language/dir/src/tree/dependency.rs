use dyst_source::StringId;

use crate::{Expression, ModuleId, Node, LocalNodeId, NodeType, LocalSymbolId};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportType {
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
    UnresolvedDefault {
        kind: DependencyKind,
        alias: StringId,
        symbol: LocalSymbolId,
    },
    /// Import or export a single item from a target (`import "foo"` or `export "foo"`).
    UnresolvedItem {
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        symbol: LocalSymbolId,
    },
    /// Value expression dependency (like `export = foo`).
    Value { value: LocalNodeId<Expression> },
    /// Internal to the module (i.e., plain exports).
    Local { symbol: LocalSymbolId },
    /// Remote to the module (i.e., imports and re-exports).
    Remote {
        symbol: LocalSymbolId,
        remote_symbol: LocalSymbolId,
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
