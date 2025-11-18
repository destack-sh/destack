use dyst_ast::StringId;

use crate::{DependencyItem, DependencyKind, ModuleId, NodeId, SymbolId};

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

/// A DependencyEdge is an edge in the dependency graph.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyEdge {
    /// Unresolved default dependency edge.
    /// Edges where the target is not found remain unresolved (we just resolve the module in place).
    UnresolvedDefault {
        kind: DependencyKind,
        target: StringId,
        module: Option<ModuleId>,
        alias: StringId,
        item: Option<NodeId<DependencyItem>>,
        source: DependencySource,
        symbol: SymbolId,
    },
    /// Unresolved item dependency edge.
    /// Edges where the target is not found remain unresolved (we just resolve the module in place).
    UnresolvedItem {
        kind: DependencyKind,
        target: StringId,
        module: Option<ModuleId>,
        name: StringId,
        alias: Option<StringId>,
        item: Option<NodeId<DependencyItem>>,
        source: DependencySource,
        symbol: SymbolId,
    },
    /// Resolved default dependency edge.
    ResolvedDefault {
        kind: DependencyKind,
        target: StringId,
        module: ModuleId,
        item: Option<NodeId<DependencyItem>>,
        source: DependencySource,
        symbol: SymbolId,
    },
    /// Resolved dependency edge.
    Resolved {
        kind: DependencyKind,
        target: StringId,
        module: ModuleId,
        item: Option<NodeId<DependencyItem>>,
        source: DependencySource,
        symbol: SymbolId,
    },
}

impl DependencyEdge {
    /// Whether the dependency edge is resolved (ignoring child nodes).
    pub fn is_resolved(&self) -> bool {
        matches!(
            self,
            DependencyEdge::Resolved { .. } | DependencyEdge::ResolvedDefault { .. }
        )
    }

    /// Whether the module is resolved.
    pub fn is_module_resolved(&self) -> bool {
        self.module().is_some()
    }

    /// Get the target name.
    pub fn target(&self) -> StringId {
        match self {
            DependencyEdge::UnresolvedDefault { target, .. } => *target,
            DependencyEdge::UnresolvedItem { target, .. } => *target,
            DependencyEdge::ResolvedDefault { target, .. } => *target,
            DependencyEdge::Resolved { target, .. } => *target,
        }
    }

    /// Get the module.
    pub fn module(&self) -> Option<ModuleId> {
        match self {
            DependencyEdge::UnresolvedDefault { module, .. } => *module,
            DependencyEdge::UnresolvedItem { module, .. } => *module,
            DependencyEdge::ResolvedDefault { module, .. } => Some(*module),
            DependencyEdge::Resolved { module, .. } => Some(*module),
        }
    }
}
