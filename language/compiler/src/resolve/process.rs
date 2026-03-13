use crate::{
    BuildKey, BuildProduct, BuildRequirement, BuildRequirementCollector, BuildRequirementError,
    BuildRequirementSet, Compiler, DiagnosticAnchor, DirReadBoundary, ResolveError,
    ResolveModuleContext, ResolveResult,
};

use destack_builtin::BuiltinLibKind;
use destack_source::ModuleId;
use std::sync::Arc;

use destack_workspace::{ArtifactKey, ModuleDirData, ModuleSource, ProfileId};

impl Compiler {
    /// Build one failed requirement set for one missing committed resolve artifact.
    fn missing_resolve_artifact_requirement(&self, key: ArtifactKey) -> BuildRequirementSet {
        let module = match &key {
            ArtifactKey::DirPrepared { module, .. } | ArtifactKey::DirResolved { module, .. } => {
                Some(*module)
            }
            _ => None,
        };
        let anchor = module
            .map(DiagnosticAnchor::from)
            .unwrap_or(DiagnosticAnchor::Global);
        let build_key = BuildKey::Artifact(key);
        let dependency = self.build_dependency_for_key(&build_key);
        let requirement = BuildRequirement::new(anchor, build_key, dependency);

        BuildRequirementSet::one(requirement)
    }

    /// Read one committed prepared DIR snapshot.
    pub(crate) fn require_artifact_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<ModuleDirData>, BuildRequirementError> {
        self.require_dir_prepared(module, profile)?;

        let Some(snapshot) = self.program.artifacts.dir_prepared(module, profile) else {
            return Err(BuildRequirementError::Failed {
                requirement: self.missing_resolve_artifact_requirement(ArtifactKey::DirPrepared {
                    module,
                    profile,
                }),
            });
        };

        Ok(snapshot)
    }

    /// Read one committed resolved DIR snapshot.
    pub(crate) fn require_artifact_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<ModuleDirData>, BuildRequirementError> {
        self.require_dir_resolved(module, profile)?;

        let Some(snapshot) = self.program.artifacts.dir_resolved(module, profile) else {
            return Err(BuildRequirementError::Failed {
                requirement: self.missing_resolve_artifact_requirement(ArtifactKey::DirResolved {
                    module,
                    profile,
                }),
            });
        };

        Ok(snapshot)
    }

    /// Build the language environment for one profile.
    pub fn process_language_environment(&self, profile: ProfileId) -> ResolveResult<BuildProduct> {
        let environment = self.resolve_language_environment(profile)?;

        Ok(BuildProduct::LanguageEnvironment(environment))
    }

    /// Build the lib environment for one profile.
    pub fn process_lib_environment(&self, profile: ProfileId) -> ResolveResult<BuildProduct> {
        let environment = self.resolve_lib_environment(profile)?;

        Ok(BuildProduct::LibEnvironment(environment))
    }

    /// Build prepared DIR for one module.
    pub fn process_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<BuildProduct> {
        let module_id = module;
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        self.resolve_module_prepare(module_id, profile, module_version, profile_version)
    }

    /// Build resolved DIR for one module.
    pub fn process_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> ResolveResult<BuildProduct> {
        let module_id = module;
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        self.require_dir_prepared(module_id, profile)
            .map_err(ResolveError::from)?;

        let module = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            ResolveModuleContext::from_module(&module)
        };
        let dir = self.require_artifact_dir_prepared(module_id, profile)?;
        let is_code_module = self.is_code_module(module_id);

        // builtin language and lib modules bootstrap the shared environments themselves
        if !matches!(
            module.source,
            ModuleSource::Builtin(BuiltinLibKind::Core | BuiltinLibKind::Std | BuiltinLibKind::Lib)
        ) {
            self.require_lib_environment(profile)
                .map_err(ResolveError::from)?;
        }
        // bootstrap selected lib lookups from prepared module surfaces once per resolve task
        else {
            let mut collector = BuildRequirementCollector::new();
            for selected_module_id in self.selected_lib_modules(profile) {
                if selected_module_id == module_id {
                    continue;
                }

                if let Err(error) = self.require_dir_prepared(selected_module_id, profile)
                    && let Some(error) = collector.try_collect::<(), _>(Err(error))
                {
                    let requirement = error.into_requirement();
                    return Err(ResolveError::UnsatisfiedRequirement { requirement });
                }
            }

            if let Some(requirement) = collector.try_into_requirement() {
                return Err(ResolveError::Yield { requirement });
            }
        }

        let (_, dir) = self.with_shared_transient_artifact_dir(
            module_id,
            profile,
            DirReadBoundary::Declared,
            dir,
            |dir| -> ResolveResult<()> {
                self.resolve_module_direct(&module, profile, dir.as_ref())?;
                self.resolve_module_canonical(&module, profile, dir.as_ref())?;
                self.update_module_graph_from_dir(
                    module_id,
                    profile,
                    module_version,
                    dir.as_ref(),
                )?;

                Ok(())
            },
        )?;

        if is_code_module {
            self.stats.record_resolve();
        }

        Ok(BuildProduct::Dir(dir.to_data()))
    }

    /// Ensure prepared DIR exists for a module.
    pub fn require_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        if self.current_build_key()
            == Some(BuildKey::Artifact(ArtifactKey::DirPrepared {
                module,
                profile,
            }))
        {
            return Ok(());
        }

        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirPrepared {
            module,
            profile,
        }))
    }

    /// Ensure another module's prepared DIR exists.
    pub fn require_dir_prepared_if_other(
        &self,
        module: ModuleId,
        other: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        if module == other {
            return Ok(());
        }

        self.require_dir_prepared(other, profile)
    }

    /// Ensure resolved DIR exists for a module.
    pub fn require_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        // avoid self dependency while resolving one module
        if self.current_build_key()
            == Some(BuildKey::Artifact(ArtifactKey::DirResolved {
                module,
                profile,
            }))
        {
            return Ok(());
        }

        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirResolved {
            module,
            profile,
        }))
    }

    /// Ensure another module's resolved DIR exists.
    pub fn require_dir_resolved_if_other(
        &self,
        module: ModuleId,
        other: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        if module == other {
            return Ok(());
        }
        self.require_dir_resolved(other, profile)
    }
}
