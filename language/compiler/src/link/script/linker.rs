use std::path::Path;
use std::sync::Arc;

use destack_artifact::{Data, DirExported, DirImported, ModuleOutput};
use destack_dir::DependencyTarget;
use destack_source::{
    File, FileId, FileType, ModuleEdge, ModuleEdgeRelation, ModuleId, PackageId, ProfileId, Span,
    StringId, TargetId,
};
use destack_workspace::{Module, ProviderContext, ProviderError, Revision, Target};

use crate::{Compiler, LinkError, LinkResult};

/// One script target linker.
pub(crate) struct ScriptLinker<'a> {
    /// The compiler driving the current link.
    pub(super) compiler: &'a Compiler,
    /// The pinned revision used by this link.
    pub(super) context: &'a dyn ProviderContext,
    /// The package directory that anchors output resolution.
    pub(super) package_dir: &'a Path,
    /// The configured root directory when one exists.
    pub(super) root_dir: Option<&'a Path>,
    /// The target being linked.
    pub(super) target: &'a Target,
    /// The target id being linked.
    pub(super) target_id: &'a TargetId,
    /// The package owning the target.
    pub(super) package_id: PackageId,
}

impl<'a> ScriptLinker<'a> {
    /// Create one script linker for one target.
    pub(crate) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        package_dir: &'a Path,
        root_dir: Option<&'a Path>,
        target: &'a Target,
        target_id: &'a TargetId,
        package_id: PackageId,
    ) -> Self {
        Self {
            compiler,
            context,
            package_dir,
            root_dir,
            target,
            target_id,
            package_id,
        }
    }

    /// Return the display name for the active target.
    pub(crate) fn target_name(&self) -> String {
        self.compiler
            .target_name(self.context.revision(), self.target_id)
    }

    /// Return the pinned revision for this link.
    pub(crate) fn revision(&self) -> Revision {
        self.context.revision()
    }

    /// Return the anchor span for one linked module.
    pub(crate) fn module_anchor_span(&self, module_id: ModuleId) -> Span {
        let module = self.compiler.module(self.revision(), module_id);

        Span::empty(module.file_id)
    }

    /// Return one revision-scoped module snapshot.
    pub(crate) fn module(&self, module_id: ModuleId) -> Arc<Module> {
        self.compiler.module(self.revision(), module_id)
    }

    /// Return one revision-scoped file snapshot.
    pub(crate) fn file(&self, file_id: FileId) -> Arc<File> {
        self.compiler.file(self.context, file_id)
    }

    /// Require the parsed AST for one linked module.
    pub(crate) fn require_ast(&self, module_id: ModuleId) -> Result<(), ProviderError> {
        self.compiler.require_ast(self.context, module_id)
    }

    /// Return one generated module output for this target.
    pub(crate) fn module_output(&self, module_id: ModuleId) -> LinkResult<Arc<ModuleOutput>> {
        self.compiler
            .module_output(self.context, module_id, self.target_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "missing module output for module {:?} target '{}': {error:?}",
                    module_id,
                    self.target_name()
                ),
            })
    }

    /// Return the resolved profile for one linked module.
    pub(crate) fn profile_id_for_module(&self, module_id: ModuleId) -> LinkResult<ProfileId> {
        self.compiler
            .target_profile_id(self.revision(), module_id, self.target_id)
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("profile not found for target '{}'", self.target_name()),
            })
    }

    /// Ensure one linked module resolves under the current target profile.
    pub(crate) fn ensure_module_profile(&self, module_id: ModuleId) -> LinkResult<()> {
        let _ = self.profile_id_for_module(module_id)?;

        Ok(())
    }

    /// Return the resolved dependency edges for one linked module.
    pub(crate) fn module_edges_for_module(
        &self,
        module_id: ModuleId,
    ) -> LinkResult<Vec<ModuleEdge>> {
        let profile_id = self.profile_id_for_module(module_id)?;
        let imported = self
            .compiler
            .require_dir_imported(self.context, module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("module imports are not ready: {error:?}"),
            })?;
        let exported = self
            .compiler
            .require_dir_exported(self.context, module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("module exports are not ready: {error:?}"),
            })?;

        Ok(module_dependency_edges(
            imported.as_ref(),
            exported.as_ref(),
        ))
    }

    /// Return the parsed data payload for one linked module.
    pub(crate) fn data(&self, module_id: ModuleId) -> LinkResult<Arc<Data>> {
        self.compiler
            .data(self.context, module_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "missing data artifact for module {:?} target '{}': {error:?}",
                    module_id,
                    self.target_name()
                ),
            })
    }

    /// Return one dependency edge with a matching relation, source site, and specifier.
    pub(crate) fn module_edge_for_site_specifier(
        &self,
        edges: &[ModuleEdge],
        relation: ModuleEdgeRelation,
        reference_site: u32,
        specifier: StringId,
    ) -> Option<ModuleEdge> {
        edges.iter().copied().find(|edge| {
            edge.relation == relation
                && edge.site == Some(reference_site)
                && edge.specifier == Some(specifier)
        })
    }

    /// Return whether one module is one plain stylesheet module.
    pub(crate) fn is_plain_stylesheet_module(&self, module_id: ModuleId) -> bool {
        let module = self.module(module_id);
        let file = self.file(module.file_id);

        file.ty == FileType::Css && !module.loader.is_file()
    }
}

/// Collect resolved module dependency edges from one module DIR surface.
fn module_dependency_edges(imported: &DirImported, exported: &DirExported) -> Vec<ModuleEdge> {
    let mut edges = Vec::new();

    // import resolutions
    for dependency in imported.dependencies.iter() {
        push_module_edge(
            &mut edges,
            Some(dependency.target),
            dependency.relation,
            Some(dependency.specifier),
            dependency.loader,
        );
    }

    // namespace exports
    for export in exported.exports.namespace_exports.iter() {
        push_module_edge(
            &mut edges,
            Some(export.target),
            ModuleEdgeRelation::NamespaceExport,
            None,
            None,
        );
    }

    edges.sort_unstable();
    edges.dedup();

    edges
}

/// Push one concrete module edge when the target is a module.
fn push_module_edge(
    edges: &mut Vec<ModuleEdge>,
    target: Option<DependencyTarget>,
    relation: ModuleEdgeRelation,
    specifier: Option<StringId>,
    loader: Option<destack_source::Loader>,
) {
    let Some(DependencyTarget::Module(module_id)) = target else {
        return;
    };

    let edge = ModuleEdge::new(module_id, relation)
        .with_specifier(specifier)
        .with_loader(loader);
    edges.push(edge);
}
