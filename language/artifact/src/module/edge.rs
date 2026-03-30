use destack_core::StringId;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::Loader;

/// The behavioral relation of one dependency edge between modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ModuleEdgeRelation {
    /// A static import-like edge.
    Import,
    /// A require-like edge.
    Require,
    /// A namespace re-export edge.
    NamespaceExport,
    /// A document module script edge.
    DocumentScript,
    /// A document stylesheet edge.
    DocumentStylesheet,
    /// A stylesheet import edge.
    StyleImport,
    /// A stylesheet URL resource edge.
    StyleUrl,
    /// A non-code resource reference edge.
    Resource,
}

impl ModuleEdgeRelation {
    /// Return true when this relation resolves through import-like conditions.
    pub fn is_import_like(self) -> bool {
        matches!(self, Self::Import)
    }

    /// Return true when this relation resolves through require-like conditions.
    pub fn is_require_like(self) -> bool {
        matches!(self, Self::Require)
    }
}

/// One dependency edge to another module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ModuleEdge {
    /// The target module id for this edge.
    pub target: ModuleId,
    /// The behavioral relation of this edge.
    pub relation: ModuleEdgeRelation,
    /// The authored module specifier when one exists.
    pub specifier: Option<StringId>,
    /// The authored loader override when one exists.
    pub loader: Option<Loader>,
}

impl ModuleEdge {
    /// Build one module edge.
    pub fn new(target: ModuleId, relation: ModuleEdgeRelation) -> Self {
        Self {
            target,
            relation,
            specifier: None,
            loader: None,
        }
    }

    /// Attach one authored specifier to this edge.
    pub fn with_specifier(mut self, specifier: Option<StringId>) -> Self {
        self.specifier = specifier;
        self
    }

    /// Attach one authored loader override to this edge.
    pub fn with_loader(mut self, loader: Option<Loader>) -> Self {
        self.loader = loader;
        self
    }

    /// Build the reverse view of this edge for one source module.
    pub fn reverse_for(self, source_module_id: ModuleId) -> Self {
        Self {
            target: source_module_id,
            relation: self.relation,
            specifier: self.specifier,
            loader: self.loader,
        }
    }
}
