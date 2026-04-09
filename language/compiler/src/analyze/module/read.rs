use destack_dir::{NodeTree, SymbolTable, TypeTable};
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, DirAnalyzed, DirBase, DirDeclared, DirElaborated, DirInterface, DirPatched,
    DirPrepared, DirResolved,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId, Revision};

use crate::analyze::common::AnalyzeIndex;
use crate::{
    ArtifactRequirement, Compiler, CompilerContext, DiagnosticAnchor, RequirementError,
    RequirementSet,
};

impl Compiler {
    /// Read one committed base DIR artifact when available.
    pub(crate) fn artifact_dir_base(&self, module_id: ModuleId) -> Option<Arc<DirBase>> {
        self.dir_base(module_id)
    }

    /// Build one failed requirement set for one missing committed artifact.
    pub(crate) fn missing_artifact_requirement(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> RequirementSet {
        let anchor = match &key {
            ArtifactKey::DirBase { module }
            | ArtifactKey::DirDeclared { module, .. }
            | ArtifactKey::DirInterface { module, .. }
            | ArtifactKey::DirAnalyzed { module, .. } => DiagnosticAnchor::from(*module),
            _ => DiagnosticAnchor::Global,
        };
        let version = self.artifact_version_for_revision(revision, &key);
        let requirement = ArtifactRequirement::new(anchor, version);

        RequirementSet::one(requirement)
    }

    /// Provide one committed remote DIR view for one exact artifact family.
    pub(crate) fn with_remote_dir_for_artifact<R>(
        &self,
        context: &CompilerContext<'_>,
        module_id: ModuleId,
        profile: ProfileId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable, &TypeTable) -> R,
    ) -> Result<R, RequirementError> {
        let remote_module = context.module(module_id);
        let remote_module = remote_module.as_ref();
        let key = artifact_key(module_id, profile);

        match key {
            ArtifactKey::DirBase { module } => {
                let snapshot = self.require_artifact_dir_base(context.revision(), module)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirPrepared { module, profile } => {
                let snapshot =
                    self.require_artifact_dir_prepared(context.revision(), module, profile)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirResolved { module, profile } => {
                let snapshot =
                    self.require_artifact_dir_resolved(context.revision(), module, profile)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirDeclared { module, profile } => {
                let snapshot =
                    self.require_artifact_dir_declared(context.revision(), module, profile)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirInterface { module, profile } => {
                let snapshot =
                    self.require_artifact_dir_interface(context.revision(), module, profile)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirAnalyzed { module, profile } => {
                let snapshot =
                    self.require_artifact_dir_analyzed(context.revision(), module, profile)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirElaborated { module, profile } => {
                let snapshot =
                    self.require_artifact_dir_elaborated(context.revision(), module, profile)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirPatched { module, profile } => {
                let snapshot =
                    self.require_artifact_dir_patched(context.revision(), module, profile)?;
                Ok(handle(
                    remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            _ => Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(context.revision(), key),
            }),
        }
    }

    /// Read one committed prepared DIR artifact.
    pub(crate) fn require_artifact_dir_prepared(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirPrepared>, RequirementError> {
        self.require_dir_prepared(revision, module_id, profile)?;

        let Some(dir) = self.dir_prepared(module_id, profile) else {
            return Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(
                    revision,
                    ArtifactKey::dir_prepared(module_id, profile),
                ),
            });
        };

        Ok(dir)
    }

    /// Read one committed base DIR artifact.
    pub(crate) fn require_artifact_dir_base(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Arc<DirBase>, RequirementError> {
        let Some(dir) = self.artifact_dir_base(module_id) else {
            return Err(RequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(revision, ArtifactKey::dir_base(module_id)),
            });
        };

        Ok(dir)
    }

    /// Read one committed resolved DIR artifact.
    pub(crate) fn require_artifact_dir_resolved(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, RequirementError> {
        self.require_dir_resolved(revision, module_id, profile)?;

        let Some(dir) = self.dir_resolved(module_id, profile) else {
            return Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(
                    revision,
                    ArtifactKey::dir_resolved(module_id, profile),
                ),
            });
        };

        Ok(dir)
    }

    /// Read one committed declared DIR artifact.
    pub(crate) fn require_artifact_dir_declared(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, RequirementError> {
        self.require_dir_declared(revision, module_id, profile)?;

        let Some(dir) = self.dir_declared(module_id, profile) else {
            return Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(
                    revision,
                    ArtifactKey::dir_declared(module_id, profile),
                ),
            });
        };

        Ok(dir)
    }

    /// Read one committed declared DIR artifact through the task-local analyze index.
    pub(crate) fn require_indexed_dir_declared(
        &self,
        index: &AnalyzeIndex,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, RequirementError> {
        if let Some(dir) = index.declared_directory(module_id, profile) {
            return Ok(dir);
        }

        let dir = self.require_artifact_dir_declared(revision, module_id, profile)?;
        index.set_declared_directory(module_id, profile, dir.clone());

        Ok(dir)
    }

    /// Read one committed interface DIR artifact.
    pub(crate) fn require_artifact_dir_interface(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirInterface>, RequirementError> {
        self.require_dir_interface(revision, module_id, profile)?;

        let Some(dir) = self.dir_interface(module_id, profile) else {
            return Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(
                    revision,
                    ArtifactKey::dir_interface(module_id, profile),
                ),
            });
        };

        Ok(dir)
    }

    /// Read one committed analyzed DIR artifact.
    pub(crate) fn require_artifact_dir_analyzed(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirAnalyzed>, RequirementError> {
        self.require_dir_analyzed(revision, module_id, profile)?;

        let Some(dir) = self.dir_analyzed(module_id, profile) else {
            return Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(
                    revision,
                    ArtifactKey::dir_analyzed(module_id, profile),
                ),
            });
        };

        Ok(dir)
    }

    /// Read one committed elaborated DIR artifact.
    pub(crate) fn require_artifact_dir_elaborated(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirElaborated>, RequirementError> {
        self.require_dir_elaborated(revision, module_id, profile)?;

        let Some(dir) = self.dir_elaborated(module_id, profile) else {
            return Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(
                    revision,
                    ArtifactKey::dir_elaborated(module_id, profile),
                ),
            });
        };

        Ok(dir)
    }

    /// Read one committed patched DIR artifact.
    pub(crate) fn require_artifact_dir_patched(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirPatched>, RequirementError> {
        self.require_dir_patched(revision, module_id, profile)?;

        let Some(dir) = self.dir_patched(module_id, profile) else {
            return Err(RequirementError::Failed {
                requirement: self.missing_artifact_requirement(
                    revision,
                    ArtifactKey::dir_patched(module_id, profile),
                ),
            });
        };

        Ok(dir)
    }

    /// Require one remote DIR artifact when a read targets a different module id.
    pub(crate) fn require_remote_artifact_dir(
        &self,
        context: &CompilerContext<'_>,
        local_module_id: ModuleId,
        target_module_id: ModuleId,
        profile: ProfileId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<(), RequirementError> {
        // local reads do not need committed artifact gating
        if local_module_id == target_module_id {
            return Ok(());
        }

        self.with_remote_dir_for_artifact(
            context,
            target_module_id,
            profile,
            artifact_key,
            |_, _, _, _| (),
        )
    }
}
