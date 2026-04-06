use crate::{
    Compiler, CompilerContext, RequirementCollector, RequirementError, ResolveError, ResolveResult,
};
use destack_artifact::{ArtifactKey, DirResolved};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};

use crate::timing::tags;

impl Compiler {
    /// Build the language environment for one profile.
    pub fn process_language_environment(
        &self,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let environment = self.resolve_language_environment(revision, profile)?;
        let artifact_key = ArtifactKey::language_environment(profile);
        context.publish_artifact(
            artifact_key,
            environment.clone(),
            |store, version, payload| store.publish_language_environment(version, payload),
        );
        context.store_artifact(
            &artifact_key,
            &environment,
            |compiler, _artifact_stamp, environment| {
                compiler.store_language_environment_image(revision, profile, environment.clone())
            },
        );

        Ok(())
    }

    /// Build the library environment for one profile.
    pub fn process_library_environment(
        &self,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let environment = self.resolve_library_environment(revision, profile)?;
        let artifact_key = ArtifactKey::library_environment(profile);
        context.publish_artifact(
            artifact_key,
            environment.clone(),
            |store, version, payload| store.publish_library_environment(version, payload),
        );
        context.store_artifact(
            &artifact_key,
            &environment,
            |compiler, _artifact_stamp, environment| {
                compiler.store_library_environment_image(revision, profile, environment.clone())
            },
        );

        Ok(())
    }

    /// Build prepared DIR for one module.
    pub fn process_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let module_id = module;
        let artifact_key = ArtifactKey::dir_prepared(module_id, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        self.resolve_module_prepare(module_id, profile, artifact_key, artifact_stamp, context)
    }

    /// Build resolved DIR for one module.
    pub fn process_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ResolveResult<()> {
        let revision = context.revision();
        let module_id = module;
        let artifact_key = ArtifactKey::dir_resolved(module_id, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        self.require_dir_prepared(revision, module_id, profile)
            .map_err(ResolveError::from)?;

        // reuse one persisted resolved dir image when available
        if let Some(dir) = context.restore_cached_artifact(
            artifact_key,
            |compiler| {
                compiler.load_dir_resolved_image(revision, module_id, artifact_stamp, profile)
            },
            |store, version, payload| store.publish_dir_resolved(version, payload),
        ) {
            self.update_module_graph_from_dir(module_id, profile, artifact_stamp, &dir, context)?;

            tracing::trace!(?module_id, ?profile, "resolve.module.cache_hit");
            return Ok(());
        }

        let module = context.module(module_id);
        let prepared = context.require_artifact_dir_prepared(module_id, profile)?;
        let mut tree = prepared.tree.as_ref().clone();
        let mut symbols = prepared.symbols.as_ref().clone();
        let mut types = prepared.types.as_ref().clone();
        let mut export_assignment = prepared.export_assignment;
        let mut namespace_exports = Vec::new();
        let mut module_binding_exports = prepared.module_binding_exports.as_ref().clone();
        let mut imported_modules = prepared.imported_modules.as_ref().clone();
        let mut exported_symbols = prepared.exported_symbols.as_ref().clone();
        let is_code_module = context.is_code_module(module_id);

        // non-builtin code modules consume the shared library environment
        if !module.is_builtin() {
            self.require_library_environment(revision, profile)
                .map_err(ResolveError::from)?;
        }
        // builtin language and library modules bootstrap selected libs directly
        else {
            let mut collector = RequirementCollector::new();
            for selected_module_id in self.selected_library_modules(profile) {
                if selected_module_id == module_id {
                    continue;
                }

                if let Err(error) = self.require_dir_prepared(revision, selected_module_id, profile)
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
                context,
                &module,
                profile,
                prepared.as_ref(),
                &mut tree,
                &mut symbols,
                &mut types,
                &mut export_assignment,
                &mut namespace_exports,
                &mut module_binding_exports,
                &mut imported_modules,
                &mut exported_symbols,
            )?;
        }

        self.resolve_module_canonical(revision, &module, profile, &mut symbols)?;

        let dir = DirResolved::from_prepared_with(
            prepared.as_ref(),
            tree,
            symbols,
            types,
            export_assignment,
            namespace_exports,
            module_binding_exports,
            imported_modules,
            exported_symbols,
        );
        self.update_module_graph_from_dir(module_id, profile, artifact_stamp, &dir, context)?;

        if is_code_module {
            self.stats.record_resolve();
        }

        context.publish_artifact(artifact_key, dir.clone(), |store, version, payload| {
            store.publish_dir_resolved(version, payload)
        });
        context.store_artifact(&artifact_key, &dir, |compiler, artifact_stamp, dir| {
            compiler.store_dir_resolved_image(revision, module_id, profile, artifact_stamp, dir)
        });

        Ok(())
    }

    /// Ensure prepared DIR exists for a module.
    pub fn require_dir_prepared(
        &self,
        revision: Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        let build_key = ArtifactKey::dir_prepared(module, profile);

        if self.current_artifact_key() == Some(build_key) {
            return Ok(());
        }

        self.require_artifact(revision, build_key)
    }

    /// Ensure another module's prepared DIR exists.
    pub fn require_dir_prepared_if_other(
        &self,
        revision: Revision,
        module: ModuleId,
        other: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        if module == other {
            return Ok(());
        }

        self.require_dir_prepared(revision, other, profile)
    }

    /// Ensure resolved DIR exists for a module.
    pub fn require_dir_resolved(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        let build_key = ArtifactKey::dir_resolved(module, profile);

        // avoid self dependency while resolving one module
        if self.current_artifact_key() == Some(build_key) {
            return Ok(());
        }

        self.require_artifact(revision, build_key)
    }

    /// Ensure another module's resolved DIR exists.
    pub fn require_dir_resolved_if_other(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
        other: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        if module == other {
            return Ok(());
        }
        self.require_dir_resolved(revision, other, profile)
    }
}
