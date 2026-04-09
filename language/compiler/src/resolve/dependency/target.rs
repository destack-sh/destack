use crate::{Compiler, RequirementCollector, ResolveError, ResolveResult};
use destack_core::StringId;
use destack_dir::{Declaration, LocalNodeId, ModuleTarget};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{PackageKind, ProfileId, Revision};

/// Reference a module binding declaration in a module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ModuleBindingReference {
    /// The module id that owns the binding.
    pub module_id: ModuleId,
    /// The declaration node for the binding.
    pub declaration: LocalNodeId<Declaration>,
}

impl Compiler {
    /// Return whether one source set contains a matching module binding.
    pub(crate) fn module_binding_exists_in_modules(
        &self,
        revision: Revision,
        module_ids: &[ModuleId],
        specifier: StringId,
    ) -> ResolveResult<bool> {
        for &module_id in module_ids {
            self.require_dir_base(revision, module_id)
                .map_err(ResolveError::from)?;
            if self.module_has_binding_specifier(revision, module_id, specifier)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Resolve a specifier to a module binding target when available.
    pub(crate) fn resolve_module_binding_target(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<ModuleTarget>> {
        let package_id = self
            .cache_module_snapshot(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?
            .package_id;
        let package_module_ids = self.package_module_ids(revision, package_id)?;
        if self.module_binding_exists_in_modules(revision, &package_module_ids, specifier)? {
            return Ok(Some(ModuleTarget::Binding(specifier)));
        }

        let ambient_module_ids = self.ambient_binding_module_ids(profile_id)?;
        if self.module_binding_exists_in_modules(revision, &ambient_module_ids, specifier)? {
            return Ok(Some(ModuleTarget::Binding(specifier)));
        }

        Ok(None)
    }

    /// Look up module bindings for a specifier.
    pub(crate) fn module_bindings_for_specifier(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<Vec<ModuleBindingReference>>> {
        let package_id = self
            .cache_module_snapshot(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?
            .package_id;
        let package_module_ids = self.package_module_ids(revision, package_id)?;
        let ambient_module_ids = self.ambient_binding_module_ids(profile_id)?;
        let mut bindings = Vec::new();

        self.collect_module_bindings_for_specifier(
            revision,
            &mut bindings,
            &package_module_ids,
            specifier,
        )?;
        self.collect_module_bindings_for_specifier(
            revision,
            &mut bindings,
            &ambient_module_ids,
            specifier,
        )?;

        if bindings.is_empty() {
            Ok(None)
        } else {
            Ok(Some(bindings))
        }
    }

    /// Collect ambient modules that can contribute module bindings.
    pub(crate) fn ambient_binding_module_ids(
        &self,
        profile_id: ProfileId,
    ) -> ResolveResult<Vec<ModuleId>> {
        self.ambient_library_modules_from_input(profile_id)
            .map(|modules| modules.iter().copied().collect())
    }

    /// Collect module ids that belong to one package.
    fn package_module_ids(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> ResolveResult<Vec<ModuleId>> {
        let package = self
            .cache_package_snapshot(revision, package_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load package snapshot: {error}"),
            })?;
        let package = package.as_ref();

        // builtin modules are a synthetic aggregate package, not one package-local binding scope
        if package.kind == PackageKind::Builtin {
            return Ok(Vec::new());
        }

        let mut module_ids = Vec::new();

        // collect modules from the target package
        let workspace_module_ids =
            self.repository
                .workspace_module_ids(revision)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to enumerate workspace modules: {error}"),
                })?;
        for module_id in workspace_module_ids {
            let module = self
                .cache_module_snapshot(revision, module_id)
                .map_err(|error| ResolveError::Internal {
                    message: format!("failed to load module snapshot: {error}"),
                })?;

            if module.package_id == package_id {
                module_ids.push(module_id);
            }
        }

        // keep traversal deterministic
        module_ids.sort_unstable();
        Ok(module_ids)
    }

    /// Collect matching bindings from one source set.
    fn collect_module_bindings_for_specifier(
        &self,
        revision: Revision,
        bindings: &mut Vec<ModuleBindingReference>,
        module_ids: &[ModuleId],
        specifier: StringId,
    ) -> ResolveResult<()> {
        let mut collector = RequirementCollector::new();

        for &module_id in module_ids {
            if let Err(error) = self.require_dir_base(revision, module_id)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                let requirement = error.into_requirement();
                return Err(ResolveError::UnsatisfiedRequirement { requirement });
            }

            if self.artifact_dir_base(module_id).is_none() {
                continue;
            }

            self.append_module_bindings_for_specifier(revision, bindings, module_id, specifier)?;
        }

        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }

    /// Return whether one module contributes a specific binding specifier.
    fn module_has_binding_specifier(
        &self,
        revision: Revision,
        module_id: ModuleId,
        specifier: StringId,
    ) -> ResolveResult<bool> {
        let module = self
            .cache_module_snapshot(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?;
        let module = module.as_ref();

        // only code modules can contribute module bindings
        if !module.is_code() {
            return Ok(false);
        }

        self.require_dir_base(revision, module_id)
            .map_err(ResolveError::from)?;
        let dir = self
            .artifact_dir_base(module_id)
            .unwrap_or_else(|| panic!("missing committed base dir artifact for {module_id:?}"));

        Ok(dir
            .module_bindings
            .iter()
            .any(|binding| binding.specifier == specifier))
    }

    /// Append one module's matching bindings to the result set.
    fn append_module_bindings_for_specifier(
        &self,
        revision: Revision,
        bindings: &mut Vec<ModuleBindingReference>,
        module_id: ModuleId,
        specifier: StringId,
    ) -> ResolveResult<()> {
        let module = self
            .cache_module_snapshot(revision, module_id)
            .map_err(|error| ResolveError::Internal {
                message: format!("failed to load module snapshot: {error}"),
            })?;
        let module = module.as_ref();

        // only code modules can contribute module bindings
        if !module.is_code() {
            return Ok(());
        }

        let dir = self
            .artifact_dir_base(module_id)
            .unwrap_or_else(|| panic!("missing committed base dir artifact for {module_id:?}"));

        for module_binding in dir.module_bindings.iter() {
            if module_binding.specifier != specifier {
                continue;
            }

            let binding_ref = ModuleBindingReference {
                module_id,
                declaration: module_binding.declaration,
            };
            if bindings.iter().any(|entry| entry == &binding_ref) {
                continue;
            }
            bindings.push(binding_ref);
        }

        Ok(())
    }
}
