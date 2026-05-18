use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, DirImported};
use destack_core::StringPool;
use destack_dir as dir;
use destack_workspace::{Module, Revision};

use crate::{ImportError, ImportResult};

/// Import phase state for one module.
pub(crate) struct ImportState<'a> {
    /// The repository revision being imported.
    pub(in crate::import) revision: Revision,
    /// The current module.
    pub(in crate::import) module: &'a Module,
    /// The shared string pool.
    pub(in crate::import) strings: &'a StringPool,
    /// The DIR view being imported.
    pub(in crate::import) view: dir::View<'a>,
    /// The dependency table being built.
    pub(in crate::import) dependencies: dir::DependencySegment,
    /// The recoverable diagnostics produced while importing.
    pub(in crate::import) diagnostics: Vec<ImportError>,
}

impl<'a> ImportState<'a> {
    /// Create import state for one module.
    pub(crate) fn new(
        revision: Revision,
        module: &'a Module,
        strings: &'a StringPool,
        view: dir::View<'a>,
    ) -> Self {
        Self {
            revision,
            module,
            strings,
            view,
            dependencies: dir::DependencySegment::new(module.id),
            diagnostics: Vec::new(),
        }
    }

    /// Finish imported DIR.
    pub(in crate::import) fn finish(self) -> (DirImported, Vec<ImportError>) {
        let imported = DirImported {
            dependencies: Arc::new(self.dependencies),
        };

        (imported, self.diagnostics)
    }

    /// Push one dependency edge.
    pub(in crate::import) fn push_dependency(&mut self, dependency: dir::DependencyEdge) {
        self.dependencies.push(dependency);
    }

    /// Push one recoverable import diagnostic.
    pub(in crate::import) fn push_diagnostic(&mut self, diagnostic: ImportError) {
        self.diagnostics.push(diagnostic);
    }

    /// Return the source anchor for one local node id.
    pub(in crate::import) fn anchor_node(&self, node_id: u32) -> ImportResult<DiagnosticAnchor> {
        self.view
            .get_span_by_id(node_id)
            .map(DiagnosticAnchor::from)
            .ok_or_else(|| ImportError::Internal {
                anchor: DiagnosticAnchor::from(self.module.id),
                message: format!("missing source span for dependency node {node_id}"),
            })
    }

    /// Return the shared string pool.
    pub(in crate::import) fn strings(&self) -> &dir::StringPool {
        self.strings
    }
}
