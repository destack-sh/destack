use std::sync::Arc;

use rustc_hash::FxHashMap;
use tspp_artifact::{
    ArtifactDependency, ConditionSet, DiagnosticAnchor, DirImported, PackageNode, ProfileKey,
    SourceDependency,
};
use tspp_core::{StringPool, closest_string};
use tspp_dir as dir;
use tspp_repository::{Environment, Module, Package, ProviderContext, Revision};
use tspp_source::{Loader, PackageId};

use crate::{ImportError, ImportResult, diagnostic_suggestion_distance};

use super::stats::ImportStats;

const IMPORT_ATTRIBUTE_TYPES: &[&str] =
    &["json", "toml", "yaml", "text", "binary", "file", "base64"];

/// Import phase state for one module.
pub(crate) struct ImportState<'a> {
    /// The repository revision being imported.
    pub(in crate::import) revision: Revision,
    /// The current module.
    pub(in crate::import) module: &'a Module,
    /// The package containing the current module.
    pub(in crate::import) package: &'a Package,
    /// The ambient environment captured by the current revision.
    pub(in crate::import) environment: &'a Environment,
    /// The active profile conditions.
    pub(in crate::import) conditions: &'a ConditionSet,
    /// The provider context recording source observations.
    pub(in crate::import) context: &'a dyn ProviderContext,
    /// Import-resolution nodes built for touched packages.
    pub(in crate::import) packages: FxHashMap<PackageId, Option<Arc<PackageNode>>>,
    /// The active profile key.
    pub(in crate::import) profile: &'a ProfileKey,
    /// The shared string pool.
    pub(in crate::import) strings: &'a StringPool,
    /// The DIR view being imported.
    pub(in crate::import) view: dir::View<'a>,
    /// The module table being built.
    pub(in crate::import) modules: dir::ModuleSegment,
    /// The recoverable diagnostics produced while importing.
    pub(in crate::import) diagnostics: Vec<ImportError>,
    /// The work stats accumulated while importing.
    pub(in crate::import) stats: ImportStats,
}

impl<'a> ImportState<'a> {
    /// Create import state for one module.
    pub(crate) fn new(
        revision: Revision,
        module: &'a Module,
        package: &'a Package,
        environment: &'a Environment,
        conditions: &'a ConditionSet,
        context: &'a dyn ProviderContext,
        profile: &'a ProfileKey,
        strings: &'a StringPool,
        view: dir::View<'a>,
    ) -> Self {
        Self {
            revision,
            module,
            package,
            environment,
            conditions,
            context,
            packages: FxHashMap::default(),
            profile,
            strings,
            view,
            modules: dir::ModuleSegment::new(module.id),
            diagnostics: Vec::new(),
            stats: ImportStats::default(),
        }
    }

    /// Finish imported DIR.
    pub(in crate::import) fn finish(self) -> (DirImported, Vec<ImportError>) {
        let imported = DirImported {
            modules: Arc::new(self.modules),
        };

        (imported, self.diagnostics)
    }

    /// Record one source observation read while resolving imports.
    pub(in crate::import) fn observe(&self, source: SourceDependency) {
        self.context.observe(ArtifactDependency::Source(source));
    }

    /// Push one module import edge.
    pub(in crate::import) fn push_module(&mut self, edge: dir::ModuleEdge) {
        self.modules.push(edge);
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
            Some(dir::ImportAttributeValue::Literal(dir::Literal::String(value))) => {
                let value = self.strings().get(*value);
                if let Some(loader) = Loader::from_type_attribute(value) {
                    Some(loader)
                } else {
                    self.report_diagnostic(ImportError::InvalidImportAttributeType {
                        anchor: anchor.clone(),
                        value: value.to_string(),
                        suggestion: closest_import_attribute_type(value),
                    });

                    None
                }
            }

            // reject non-string loader names
            Some(_) => {
                self.report_diagnostic(ImportError::InvalidImportAttributeType {
                    anchor: anchor.clone(),
                    value: "<non-string>".to_string(),
                    suggestion: None,
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

/// Return the closest supported import attribute type.
fn closest_import_attribute_type(value: &str) -> Option<String> {
    let candidates = IMPORT_ATTRIBUTE_TYPES
        .iter()
        .map(|candidate| (*candidate).to_string());

    closest_string(value, candidates, diagnostic_suggestion_distance(value))
}
