use crate::{
    BuildKey, BuildRequirementCollector, BuildRequirementError, Compiler, ResolveError,
    ResolveResult,
};

use destack_builtin::BuiltinLibKind;
use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, DirResolved, ModuleSource, ProfileId};

use crate::timing::tags;

impl Compiler {
    /// Build the language environment for one profile.
    pub fn process_language_environment(&self, profile: ProfileId) -> ResolveResult<()> {
        let environment = self.resolve_language_environment(profile)?;
        self.program
            .artifacts
            .publish(ArtifactKey::language_environment(profile), environment);

        Ok(())
    }

    /// Build the lib environment for one profile.
    pub fn process_lib_environment(&self, profile: ProfileId) -> ResolveResult<()> {
        let environment = self.resolve_lib_environment(profile)?;
        self.program
            .artifacts
            .publish(ArtifactKey::lib_environment(profile), environment);

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
        self.resolve_module_prepare(module_id, profile, module_version, profile_version)
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

        let module = {
            let module = self.program.modules.get(module_id);
            module
        };
        let prepared = self.require_artifact_dir_prepared(module_id, profile)?;
        let mut tree = prepared.tree.as_ref().clone();
        let mut symbols = prepared.symbols.as_ref().clone();
        let mut export_assignment = prepared.export_assignment;
        let mut namespace_exports = Vec::new();
        let mut module_binding_exports = prepared.module_binding_exports.as_ref().clone();
        let mut imported_modules = prepared.imported_modules.as_ref().clone();
        let mut exported_symbols = prepared.exported_symbols.as_ref().clone();
        let is_code_module = self.is_code_module(module_id);

        // prepare the consuming package binding table before dependency resolution
        if is_code_module && !module.is_builtin() {
            self.require_module_binding_sources(module.package_id, profile)?;
            let _ = self.module_binding_table_for_module(module_id, profile)?;
        }

        // builtin language and lib modules bootstrap the shared environments themselves
        if !matches!(
            module.source,
            ModuleSource::Builtin(
                BuiltinLibKind::Intrinsic | BuiltinLibKind::Language | BuiltinLibKind::Library
            )
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

        // direct resolve
        if is_code_module {
            let _timing = self.timing_scope(tags::RESOLVE_MODULE_DIRECT);
            self.resolve_module_direct(
                &module,
                profile,
                prepared.as_ref(),
                &mut tree,
                &mut symbols,
                &mut export_assignment,
                &mut namespace_exports,
                &mut module_binding_exports,
                &mut imported_modules,
                &mut exported_symbols,
            )?;
        }

        self.resolve_module_canonical(&module, profile, &mut symbols)?;

        let dir = DirResolved::from_prepared_with(
            prepared.as_ref(),
            tree,
            symbols,
            export_assignment,
            namespace_exports,
            module_binding_exports,
            imported_modules,
            exported_symbols,
        );
        self.update_module_graph_from_dir(module_id, profile, module_version, &dir)?;

        if is_code_module {
            self.stats.record_resolve();
        }

        self.program
            .artifacts
            .publish(ArtifactKey::dir_resolved(module_id, profile), dir);

        Ok(())
    }

    /// Ensure prepared DIR exists for a module.
    pub fn require_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        let build_key = BuildKey::artifact(ArtifactKey::dir_prepared(module, profile));

        if self.current_build_key() == Some(build_key.clone()) {
            return Ok(());
        }

        self.require_build_key(build_key)
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
        let build_key = BuildKey::artifact(ArtifactKey::dir_resolved(module, profile));

        // avoid self dependency while resolving one module
        if self.current_build_key() == Some(build_key.clone()) {
            return Ok(());
        }

        self.require_build_key(build_key)
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
