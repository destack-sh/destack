use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Loader, ModuleEdgeRelation, ModuleId};

/// One resolved relation from a source module to another source module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ModuleEdge {
    /// The target module.
    pub target: ModuleId,
    /// The source relation.
    pub relation: ModuleEdgeRelation,
    /// The authored specifier when the relation has one.
    pub specifier: Option<StringId>,
    /// The local source site when the relation came from a concrete document or style node.
    pub site: Option<u32>,
    /// The requested loader override when the relation has one.
    pub loader: Option<Loader>,
}

impl ModuleEdge {
    /// Create one module edge.
    pub fn new(target: ModuleId, relation: ModuleEdgeRelation) -> Self {
        Self {
            target,
            relation,
            specifier: None,
            site: None,
            loader: None,
        }
    }

    /// Return this edge with an authored specifier.
    pub fn with_specifier(mut self, specifier: Option<StringId>) -> Self {
        self.specifier = specifier;
        self
    }

    /// Return this edge with a local source site.
    pub fn with_site(mut self, site: Option<u32>) -> Self {
        self.site = site;
        self
    }

    /// Return this edge with a loader override.
    pub fn with_loader(mut self, loader: Option<Loader>) -> Self {
        self.loader = loader;
        self
    }
}
