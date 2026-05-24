use std::path::Path;
use std::sync::Arc;

use destack_artifact::{Data, DirExported, ModuleOutput};
use destack_dir as dir;
use destack_source::{
    File, FileId, FileType, ModuleId, PackageId, ProfileId, Span, StringId, TargetId,
};
use destack_workspace::{Module, ProviderContext, Revision, Target};

use crate::{Compiler, CompilerError, LinkError, LinkResult};

use super::{ModuleEdge, ModuleRelation};

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
    /// The target name selected by the target id.
    pub(super) target_name: String,
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
    ) -> LinkResult<Self> {
        let target_name = compiler
            .target_name(context.revision(), *target_id)
            .map_err(|error| Compiler::link_error(package_id, error))?;

        Ok(Self {
            compiler,
            context,
            package_dir,
            root_dir,
            target,
            target_id,
            target_name,
            package_id,
        })
    }

    /// Return the active target name.
    pub(crate) fn target_name(&self) -> &str {
        &self.target_name
    }

    /// Return the pinned revision for this link.
    pub(crate) fn revision(&self) -> Revision {
        self.context.revision()
    }

    /// Return the anchor span for one linked module.
    pub(crate) fn module_anchor_span(&self, module_id: ModuleId) -> LinkResult<Span> {
        let module = self.module(module_id)?;

        Ok(Span::empty(module.file_id))
    }

    /// Return one revision-scoped module snapshot.
    pub(crate) fn module(&self, module_id: ModuleId) -> LinkResult<Arc<Module>> {
        self.compiler
            .module(self.revision(), module_id)
            .map_err(|error| self.link_error(error))
    }

    /// Return one revision-scoped file snapshot.
    pub(crate) fn file(&self, file_id: FileId) -> LinkResult<Arc<File>> {
        self.compiler
            .file(self.context, file_id)
            .map_err(|error| self.link_error(error))
    }

    /// Return one generated module output for this target.
    pub(crate) fn module_output(&self, module_id: ModuleId) -> LinkResult<Arc<ModuleOutput>> {
        self.compiler
            .artifact_reader(self.context)
            .module_output(module_id, *self.target_id)
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
            .map_err(|error| self.link_error(error))?
            .ok_or_else(|| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("profile not found for target '{}'", self.target_name()),
            })
    }

    /// Ensure one linked module resolves under the current target profile.
    pub(crate) fn ensure_target_profile(&self, module_id: ModuleId) -> LinkResult<()> {
        let _ = self.profile_id_for_module(module_id)?;

        Ok(())
    }

    /// Return the resolved module edges for one linked module.
    pub(crate) fn module_edges_for_module(
        &self,
        module_id: ModuleId,
    ) -> LinkResult<Vec<ModuleEdge>> {
        let profile_id = self.profile_id_for_module(module_id)?;
        let artifacts = self.compiler.artifact_reader(self.context);
        let imported = artifacts
            .dir_imported(module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("module imports are not ready: {error:?}"),
            })?;
        let expanded = artifacts
            .dir_expanded(module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("module expansion is not ready: {error:?}"),
            })?;
        let exported = artifacts
            .dir_exported(module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!("module exports are not ready: {error:?}"),
            })?;
        let modules = expanded.module_table(&imported);

        Ok(module_import_edges(&modules, exported.as_ref()))
    }

    /// Return the parsed data payload for one linked module.
    pub(crate) fn data(&self, module_id: ModuleId) -> LinkResult<Arc<Data>> {
        self.compiler
            .artifact_reader(self.context)
            .data(module_id)
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

    /// Map one compiler boundary failure into a link diagnostic.
    pub(crate) fn link_error(&self, error: CompilerError) -> LinkError {
        Compiler::link_error(self.package_id, error)
    }

    /// Return one module edge with a matching relation, source site, and specifier.
    pub(crate) fn module_edge_for_site_specifier(
        &self,
        edges: &[ModuleEdge],
        relation: ModuleRelation,
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
    pub(crate) fn is_plain_stylesheet_module(&self, module_id: ModuleId) -> LinkResult<bool> {
        let module = self.module(module_id)?;
        let file = self.file(module.file_id)?;

        Ok(file.ty == FileType::Css && !module.loader.is_file())
    }
}

/// Collect resolved module edges from one module DIR surface.
fn module_import_edges(modules: &dir::ModuleTable<'_>, exported: &DirExported) -> Vec<ModuleEdge> {
    let mut edges = Vec::new();

    // import resolutions
    for module in modules.iter() {
        push_module_edge(
            &mut edges,
            module.target,
            module_relation(module.relation),
            Some(module.specifier),
            module.loader,
        );
    }

    // re-export edges
    for export in exported.exports.star_exports() {
        push_module_edge(
            &mut edges,
            export.target,
            ModuleRelation::ReExport,
            None,
            None,
        );
    }

    edges.sort_unstable();
    edges.dedup();

    edges
}

/// Return the script-linker relation for one DIR module relation.
fn module_relation(relation: dir::ModuleRelation) -> ModuleRelation {
    match relation {
        dir::ModuleRelation::Import => ModuleRelation::Import,
        dir::ModuleRelation::ReExport => ModuleRelation::ReExport,
    }
}

/// Push one concrete module edge when the target is a module.
fn push_module_edge(
    edges: &mut Vec<ModuleEdge>,
    target: Option<ModuleId>,
    relation: ModuleRelation,
    specifier: Option<StringId>,
    loader: Option<destack_source::Loader>,
) {
    let Some(module_id) = target else {
        return;
    };

    let edge = ModuleEdge::new(module_id, relation)
        .with_specifier(specifier)
        .with_loader(loader);
    edges.push(edge);
}
