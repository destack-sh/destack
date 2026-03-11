use crate::{BuildKey, BuildRequirementError, Compiler, ResolveError, ResolveResult};

use destack_builtin::BuiltinLibKind;
use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ModuleSource, ProfileId};

impl Compiler {
    /// Build the language environment for one profile.
    pub fn process_language_environment(&self, profile: ProfileId) -> ResolveResult<()> {
        self.resolve_language_environment(profile)?;

        Ok(())
    }

    /// Build the lib environment for one profile.
    pub fn process_lib_environment(&self, profile: ProfileId) -> ResolveResult<()> {
        self.resolve_lib_environment(profile)?;

        Ok(())
    }

    /// Build prepared DIR for one module.
    pub fn process_dir_prepared(&self, module: ModuleId, profile: ProfileId) -> ResolveResult<()> {
        let module_id = module;
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<ResolveError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        self.resolve_module_prepare(module_id, profile, module_version, profile_version)?;

        Ok(())
    }

    /// Build resolved DIR for one module.
    pub fn process_dir_resolved(&self, module: ModuleId, profile: ProfileId) -> ResolveResult<()> {
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

        let module = self.program.modules.get(module_id);
        let module = module.read();

        // builtin language and lib modules bootstrap the shared environments themselves
        if !matches!(
            module.source,
            ModuleSource::Builtin(BuiltinLibKind::Core | BuiltinLibKind::Std | BuiltinLibKind::Lib)
        ) {
            self.require_lib_environment(profile)
                .map_err(ResolveError::from)?;
        }

        self.resolve_module_direct(module_id, profile, module_version, profile_version)?;
        self.resolve_module_canonical(module_id, profile, module_version, profile_version)?;
        if self.is_code_module(module_id) {
            self.stats.record_resolve();
        }

        Ok(())
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
