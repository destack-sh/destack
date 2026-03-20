use destack_dir::{NodeTree, SymbolTable, TypeTable};
use std::sync::Arc;

use destack_source::ModuleId;
use destack_workspace::{
    ArtifactKey, DirAnalyzed, DirBase, DirDeclared, DirElaborated, DirInterface, DirPatched,
    DirPrepared, DirResolved, Module, ProfileId,
};

use crate::{
    ArtifactRequirement, ArtifactRequirementError, ArtifactRequirementSet, Compiler,
    DiagnosticAnchor,
};

impl Compiler {
    /// Read one committed base DIR artifact when available.
    pub(crate) fn artifact_dir_base(&self, module_id: ModuleId) -> Option<Arc<DirBase>> {
        self.program.artifacts.dir_base(module_id)
    }

    /// Build one failed requirement set for one missing committed artifact.
    fn missing_artifact_requirement(&self, key: ArtifactKey) -> ArtifactRequirementSet {
        let anchor = match &key {
            ArtifactKey::DirBase { module }
            | ArtifactKey::DirDeclared { module, .. }
            | ArtifactKey::DirInterface { module, .. }
            | ArtifactKey::DirAnalyzed { module, .. } => DiagnosticAnchor::from(*module),
            _ => DiagnosticAnchor::Global,
        };
        let dependency = self.artifact_dependency_for_key(&key);
        let requirement = ArtifactRequirement::new(anchor, key, dependency);

        ArtifactRequirementSet::one(requirement)
    }

    /// Provide one committed remote DIR view for one exact artifact family.
    pub(crate) fn with_remote_dir_for_artifact<R>(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
        handle: impl FnOnce(&Module, &NodeTree, &SymbolTable, &TypeTable) -> R,
    ) -> Result<R, ArtifactRequirementError> {
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.as_ref();
        let key = artifact_key(module_id, profile);

        match key {
            ArtifactKey::DirBase { module } => {
                let snapshot = self.require_artifact_dir_base(module)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirPrepared { module, profile } => {
                let snapshot = self.require_artifact_dir_prepared(module, profile)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirResolved { module, profile } => {
                let snapshot = self.require_artifact_dir_resolved(module, profile)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirDeclared { module, profile } => {
                let snapshot = self.require_artifact_dir_declared(module, profile)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirInterface { module, profile } => {
                let snapshot = self.require_artifact_dir_interface(module, profile)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirAnalyzed { module, profile } => {
                let snapshot = self.require_artifact_dir_analyzed(module, profile)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirElaborated { module, profile } => {
                let snapshot = self.require_artifact_dir_elaborated(module, profile)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            ArtifactKey::DirPatched { module, profile } => {
                let snapshot = self.require_artifact_dir_patched(module, profile)?;
                Ok(handle(
                    &remote_module,
                    &snapshot.tree,
                    &snapshot.symbols,
                    &snapshot.types,
                ))
            }
            _ => Err(ArtifactRequirementError::Failed {
                requirement: self.missing_artifact_requirement(key),
            }),
        }
    }

    /// Read one committed prepared DIR artifact.
    pub(crate) fn require_artifact_dir_prepared(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirPrepared>, ArtifactRequirementError> {
        self.require_dir_prepared(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_prepared(module_id, profile) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::dir_prepared(module_id, profile)),
            });
        };

        Ok(dir)
    }

    /// Read one committed base DIR artifact.
    pub(crate) fn require_artifact_dir_base(
        &self,
        module_id: ModuleId,
    ) -> Result<Arc<DirBase>, ArtifactRequirementError> {
        let Some(dir) = self.artifact_dir_base(module_id) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self.missing_artifact_requirement(ArtifactKey::dir_base(module_id)),
            });
        };

        Ok(dir)
    }

    /// Read one committed resolved DIR artifact.
    pub(crate) fn require_artifact_dir_resolved(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, ArtifactRequirementError> {
        self.require_dir_resolved(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_resolved(module_id, profile) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::dir_resolved(module_id, profile)),
            });
        };

        Ok(dir)
    }

    /// Read one committed declared DIR artifact.
    pub(crate) fn require_artifact_dir_declared(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, ArtifactRequirementError> {
        self.require_dir_declared(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_declared(module_id, profile) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::dir_declared(module_id, profile)),
            });
        };

        Ok(dir)
    }

    /// Read one committed interface DIR artifact.
    pub(crate) fn require_artifact_dir_interface(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirInterface>, ArtifactRequirementError> {
        self.require_dir_interface(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_interface(module_id, profile) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::dir_interface(module_id, profile)),
            });
        };

        Ok(dir)
    }

    /// Read one committed analyzed DIR artifact.
    pub(crate) fn require_artifact_dir_analyzed(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirAnalyzed>, ArtifactRequirementError> {
        self.require_dir_analyzed(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_analyzed(module_id, profile) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::dir_analyzed(module_id, profile)),
            });
        };

        Ok(dir)
    }

    /// Read one committed elaborated DIR artifact.
    pub(crate) fn require_artifact_dir_elaborated(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirElaborated>, ArtifactRequirementError> {
        self.require_dir_elaborated(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_elaborated(module_id, profile) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::dir_elaborated(module_id, profile)),
            });
        };

        Ok(dir)
    }

    /// Read one committed patched DIR artifact.
    pub(crate) fn require_artifact_dir_patched(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirPatched>, ArtifactRequirementError> {
        self.require_dir_patched(module_id, profile)?;

        let Some(dir) = self.program.artifacts.dir_patched(module_id, profile) else {
            return Err(ArtifactRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::dir_patched(module_id, profile)),
            });
        };

        Ok(dir)
    }

    /// Require one remote DIR artifact when a read targets a different module id.
    pub(crate) fn require_remote_artifact_dir(
        &self,
        local_module_id: ModuleId,
        target_module_id: ModuleId,
        profile: ProfileId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<(), ArtifactRequirementError> {
        // local reads do not need committed artifact gating
        if local_module_id == target_module_id {
            return Ok(());
        }

        self.with_remote_dir_for_artifact(target_module_id, profile, artifact_key, |_, _, _, _| ())
    }
}
