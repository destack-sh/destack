use std::path::Path;
use std::sync::Arc;

use destack_artifact::{Data, ModuleOutput};
use destack_repository::{Module, ProviderContext, Revision, Target};
use destack_source::{File, FileId, ModuleId, PackageId, ProfileId, Span, TargetId};

use crate::{Compiler, CompilerError, LinkError, LinkResult};

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
            .profile_id_for_target(self.revision(), module_id, self.target_id)
            .map_err(|error| self.link_error(error))
    }

    /// Ensure one linked module resolves under the current target profile.
    pub(crate) fn ensure_profile_for_target(&self, module_id: ModuleId) -> LinkResult<()> {
        let _ = self.profile_id_for_module(module_id)?;

        Ok(())
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
}
