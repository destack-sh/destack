use dyst_source::StringId;

use crate::{ModuleId, Node, NodeType, SymbolId};

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

/// The source of the import.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencySource {
    /// Plain import statement (like `import "foo"`).
    ImportStatement,
    /// Re-export statement (like `export { bar } from "foo"`).
    ReExportStatement,
    /// Import call (like `await import("foo")`).
    ImportCall,
    /// Require call (like `require("foo")`).
    RequireCall,
}

impl DependencySource {
    /// Whether the source is dynamic (like `await import("foo")` or `require("foo")`).
    pub fn is_dynamic(&self) -> bool {
        matches!(
            self,
            DependencySource::ImportCall | DependencySource::RequireCall
        )
    }
}

/// A DependencyItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyItem {
    /// Unresolved default from a target (like `import * as foo from "foo"`).
    UnresolvedDefault {
        kind: DependencyKind,
        alias: StringId,
        local_symbol: SymbolId,
    },
    /// Import or export a single item from a target (`import "foo"` or `export "foo"`).
    UnresolvedItem {
        kind: DependencyKind,
        name: StringId,
        alias: Option<StringId>,
        local_symbol: SymbolId,
    },
    /// Internal to the module.
    Local { local_symbol: SymbolId },
    /// Remote to the module.
    Remote {
        local_symbol: SymbolId,
        remote_symbol: SymbolId,
        module: ModuleId,
    },
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}

impl DependencyItem {
    /// Whether the dependency item is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        matches!(
            self,
            DependencyItem::Local { .. } | DependencyItem::Remote { .. }
        )
    }
}
