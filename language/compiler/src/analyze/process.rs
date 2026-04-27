use destack_artifact::{ArtifactKey, DirAnalyzed, DirDeclared, DirInterface, IntrinsicEnvironment};
use destack_dir::CaptureTable;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};

use crate::{AnalyzeError, AnalyzeResult, Compiler, CompilerContext, RequirementError};

impl Compiler {
    /// Map one build requirement failure into an analyze error.
    pub(crate) fn analyze_error_from_requirement(&self, error: RequirementError) -> AnalyzeError {
        match error {
            RequirementError::NotReady { requirement } => AnalyzeError::Yield { requirement },
            RequirementError::Failed { requirement } => {
                AnalyzeError::UnsatisfiedRequirement { requirement }
            }
        }
    }

    /// Build the intrinsic environment for one profile.
    pub(crate) fn process_intrinsic_environment(
        &self,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::intrinsic_environment(profile);

        // publish the currently authoritative intrinsic environment
        let environment = IntrinsicEnvironment::default();
        context.publish_artifact(
            artifact_key,
            environment.clone(),
            |store, version, payload| store.publish_intrinsic_environment(version, payload),
        );
        context.store_artifact(
            &artifact_key,
            &environment,
            |compiler, _artifact_stamp, environment| {
                compiler.store_intrinsic_environment_image(revision, profile, environment.clone())
            },
        );

        Ok(())
    }

    /// Require the intrinsic environment for one profile.
    pub(crate) fn require_intrinsic_environment(
        &self,
        revision: Revision,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::intrinsic_environment(profile))
    }

    /// Ensure declared DIR exists for one module.
    pub fn require_dir_declared(
        &self,
        revision: Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::dir_declared(module, profile))
    }

    /// Ensure analyzed DIR exists for one module.
    pub fn require_dir_analyzed(
        &self,
        revision: Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::dir_analyzed(module, profile))
    }

    /// Build declared DIR for one module.
    pub fn process_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_declared(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted declared dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_declared_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_declared(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // declared DIR currently preserves resolved symbols and starts capture ownership
        let resolved = context
            .require_artifact_dir_resolved(module, profile)
            .map_err(|error| self.analyze_error_from_requirement(error))?;
        let symbols = resolved.symbols.as_ref().clone();
        let types = resolved.types.as_ref().clone();
        let captures = CaptureTable::new();
        let payload = DirDeclared::from_resolved_with(resolved.as_ref(), symbols, types, captures);

        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_dir_declared(version, payload)
        });
        context.store_artifact(
            &artifact_key,
            &payload,
            |compiler, artifact_stamp, payload| {
                compiler.store_dir_declared_image(
                    revision,
                    module,
                    profile,
                    artifact_stamp,
                    payload,
                )
            },
        );

        Ok(())
    }

    /// Build interface DIR for one module.
    pub fn process_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_interface(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted interface dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_interface_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_interface(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // interface DIR is the exported resolved surface over declared locals
        let resolved = context
            .require_artifact_dir_resolved(module, profile)
            .map_err(|error| self.analyze_error_from_requirement(error))?;
        let declared = context
            .require_artifact_dir_declared(module, profile)
            .map_err(|error| self.analyze_error_from_requirement(error))?;
        let payload =
            DirInterface::from_resolved_and_declared(resolved.as_ref(), declared.as_ref());

        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_dir_interface(version, payload)
        });
        context.store_artifact(
            &artifact_key,
            &payload,
            |compiler, artifact_stamp, payload| {
                compiler.store_dir_interface_image(
                    revision,
                    module,
                    profile,
                    artifact_stamp,
                    payload,
                )
            },
        );

        Ok(())
    }

    /// Build analyzed DIR for one module.
    pub fn process_dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_analyzed(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted analyzed dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_analyzed_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_analyzed(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        // analyzed DIR preserves interface types until the next semantic model exists
        let interface = context
            .require_artifact_dir_interface(module, profile)
            .map_err(|error| self.analyze_error_from_requirement(error))?;
        let declared = context
            .require_artifact_dir_declared(module, profile)
            .map_err(|error| self.analyze_error_from_requirement(error))?;
        let types = interface.types.as_ref().clone();
        let captures = declared.captures.as_ref().clone();
        let payload = DirAnalyzed::from_interface_and_declared_with(
            interface.as_ref(),
            declared.as_ref(),
            types,
            captures,
        );

        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_dir_analyzed(version, payload)
        });
        context.store_artifact(
            &artifact_key,
            &payload,
            |compiler, artifact_stamp, payload| {
                compiler.store_dir_analyzed_image(
                    revision,
                    module,
                    profile,
                    artifact_stamp,
                    payload,
                )
            },
        );

        Ok(())
    }
}
