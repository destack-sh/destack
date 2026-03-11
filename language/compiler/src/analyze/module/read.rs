use std::sync::Arc;

use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ModuleDirData, ProfileId};

use crate::{
    BuildKey, BuildRequirement, BuildRequirementError, BuildRequirementSet, Compiler,
    DiagnosticAnchor,
};

/// The artifact boundary required for one cross-module analyze read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirReadBoundary {
    /// The declared module surface.
    Declared,
    /// The published interface surface.
    Interface,
    /// The fully analyzed module state.
    Analyzed,
}

impl Compiler {
    /// Build one failed requirement set for one missing committed artifact.
    fn missing_artifact_requirement(&self, key: ArtifactKey) -> BuildRequirementSet {
        let anchor = match &key {
            ArtifactKey::DirBase { module }
            | ArtifactKey::DirDeclared { module, .. }
            | ArtifactKey::DirInterface { module, .. }
            | ArtifactKey::DirAnalyzed { module, .. } => DiagnosticAnchor::from(*module),
            _ => DiagnosticAnchor::Global,
        };
        let dependency = self.build_dependency_for_key(&BuildKey::Artifact(key.clone()));
        let requirement = BuildRequirement::new(anchor, BuildKey::Artifact(key), dependency);

        BuildRequirementSet::one(requirement)
    }

    /// Read one committed DIR snapshot for one required boundary.
    pub(crate) fn require_artifact_dir_for_boundary(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        boundary: DirReadBoundary,
    ) -> Result<Arc<ModuleDirData>, BuildRequirementError> {
        self.require_module_boundary_for_read(module_id, profile, boundary)?;

        let snapshot = match boundary {
            DirReadBoundary::Declared => self.program.artifacts.dir_declared(module_id, profile),
            DirReadBoundary::Interface => self.program.artifacts.dir_interface(module_id, profile),
            DirReadBoundary::Analyzed => self.program.artifacts.dir_analyzed(module_id, profile),
        };

        let Some(snapshot) = snapshot else {
            let key = match boundary {
                DirReadBoundary::Declared => ArtifactKey::DirDeclared {
                    module: module_id,
                    profile,
                },
                DirReadBoundary::Interface => ArtifactKey::DirInterface {
                    module: module_id,
                    profile,
                },
                DirReadBoundary::Analyzed => ArtifactKey::DirAnalyzed {
                    module: module_id,
                    profile,
                },
            };

            return Err(BuildRequirementError::Failed {
                requirement: self.missing_artifact_requirement(key),
            });
        };

        Ok(snapshot)
    }

    /// Read one committed base DIR snapshot.
    pub(crate) fn require_artifact_dir_base(
        &self,
        module_id: ModuleId,
    ) -> Result<Arc<ModuleDirData>, BuildRequirementError> {
        let Some(snapshot) = self.program.artifacts.dir_base(module_id) else {
            return Err(BuildRequirementError::Failed {
                requirement: self
                    .missing_artifact_requirement(ArtifactKey::DirBase { module: module_id }),
            });
        };

        Ok(snapshot)
    }

    /// Require one artifact boundary when a read targets a different module id.
    pub(crate) fn require_boundary_for_remote_module_read(
        &self,
        local_module_id: ModuleId,
        target_module_id: ModuleId,
        profile: ProfileId,
        boundary: DirReadBoundary,
    ) -> Result<(), BuildRequirementError> {
        // local reads do not need boundary gating
        if local_module_id == target_module_id {
            return Ok(());
        }

        self.require_module_boundary_for_read(target_module_id, profile, boundary)
    }

    /// Ensure one module satisfies the artifact boundary for one cross-module read.
    pub(crate) fn require_module_boundary_for_read(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        boundary: DirReadBoundary,
    ) -> Result<(), BuildRequirementError> {
        // gate reads by artifact boundary
        match boundary {
            DirReadBoundary::Declared => self.require_dir_declared(module_id, profile),
            DirReadBoundary::Interface => self.require_dir_interface(module_id, profile),
            DirReadBoundary::Analyzed => self.require_dir_analyzed(module_id, profile),
        }
    }
}
