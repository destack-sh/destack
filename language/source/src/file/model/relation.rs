use serde::{Deserialize, Serialize};

/// The semantic relation between two source modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ModuleEdgeRelation {
    /// Static import.
    Import,
    /// CommonJS require.
    Require,
    /// Namespace re-export.
    NamespaceExport,
    /// Document module script.
    DocumentScript,
    /// Document stylesheet.
    DocumentStylesheet,
    /// Stylesheet import.
    StyleImport,
    /// Stylesheet URL resource.
    StyleUrl,
    /// Non-code resource reference.
    Resource,
}

impl ModuleEdgeRelation {
    /// Return whether this relation resolves through import-like conditions.
    pub fn is_import_like(self) -> bool {
        matches!(self, Self::Import)
    }

    /// Return whether this relation resolves through require-like conditions.
    pub fn is_require_like(self) -> bool {
        matches!(self, Self::Require)
    }
}
