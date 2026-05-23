use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, DirImported};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::Loader;
use destack_workspace::{ConditionSet, Module, Revision};

use crate::{ImportError, ImportResult};

/// Import phase state for one module.
pub(crate) struct ImportState<'a> {
    /// The repository revision being imported.
    pub(in crate::import) revision: Revision,
    /// The current module.
    pub(in crate::import) module: &'a Module,
    /// The active source graph conditions.
    pub(in crate::import) conditions: &'a ConditionSet,
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
        conditions: &'a ConditionSet,
        strings: &'a StringPool,
        view: dir::View<'a>,
    ) -> Self {
        Self {
            revision,
            module,
            conditions,
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

    /// Report one recoverable import diagnostic.
    pub(in crate::import) fn report_diagnostic(&mut self, diagnostic: ImportError) {
        self.diagnostics.push(diagnostic);
    }

    /// Read the loader selected by one import attribute clause.
    pub(in crate::import) fn extract_module_loader(
        &mut self,
        anchor: &DiagnosticAnchor,
        attributes: Option<&dir::ImportAttributeClause>,
    ) -> Option<Loader> {
        let loader_attribute = attributes.and_then(|attributes| {
            attributes.attributes.iter().find(|attribute| {
                let key = self.strings().get(attribute.key.string());
                key == "type"
            })
        });

        match loader_attribute.map(|attribute| &attribute.value) {
            // accept known loader names
            Some(dir::ImportAttributeValue::ScalarLiteral(dir::ScalarLiteral::String(value))) => {
                let value = self.strings().get(*value);
                if let Some(loader) = Loader::from_type_attribute(value) {
                    Some(loader)
                } else {
                    self.report_diagnostic(ImportError::InvalidImportAttributeType {
                        anchor: anchor.clone(),
                        value: value.to_string(),
                    });

                    None
                }
            }

            // reject non-string loader names
            Some(_) => {
                self.report_diagnostic(ImportError::InvalidImportAttributeType {
                    anchor: anchor.clone(),
                    value: "<non-string>".to_string(),
                });

                None
            }

            // use repository inference
            None => None,
        }
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
