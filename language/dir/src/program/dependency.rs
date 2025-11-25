use dyst_ast::StringId;

use crate::{
    DependencyItem, DependencyKind, DependencyMode, GlobalSymbolId, LocalNodeId, LocalSymbolId,
    ModuleId,
};

/// The source of the import.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
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
pub struct DependencyEdge {
    /// The mode of the dependency (item, default, namespace).
    pub mode: DependencyMode,
    /// The kind of the dependency (value or type).
    pub kind: DependencyKind,
    /// The unresolved target of the edge.
    pub target: StringId,
    /// The resolved module of the edge.
    pub module: Option<ModuleId>,
    /// The corresponding item in the tree.
    pub item: Option<LocalNodeId<DependencyItem>>,
    /// The source of the edge.
    pub source: DependencySource,
    /// The local symbol of the edge.
    pub symbol: Option<LocalSymbolId>,
    /// The global symbol of the target.
    pub target_symbol: Option<GlobalSymbolId>,
}
