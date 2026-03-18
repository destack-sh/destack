use crate::{BuildRequirementCollector, Compiler, ResolveError, ResolveResult};
use destack_core::StringId;
use destack_dir::{Declaration, LocalNodeId, ModuleTarget};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{ModuleFormat, ProfileId};
use indexmap::IndexMap;

/// Reference a module binding declaration in a module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ModuleBindingReference {
    /// The module id that owns the binding.
    pub module_id: ModuleId,
    /// The declaration node for the binding.
    pub declaration: LocalNodeId<Declaration>,
}

/// Track module bindings reachable from a root set.
#[derive(Debug, Clone)]
pub(crate) struct ModuleBindingTable {
    /// Module bindings by specifier.
    pub bindings_by_specifier: IndexMap<StringId, Vec<ModuleBindingReference>>,
}

impl Default for ModuleBindingTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleBindingTable {
    /// Create an empty table.
    fn new() -> Self {
        Self {
            bindings_by_specifier: IndexMap::new(),
        }
    }
}

impl Compiler {
    /// Return one cached module binding table for a profile when one already exists.
    fn cached_module_binding_table_for_profile(
        &self,
        profile_id: ProfileId,
    ) -> Option<ModuleBindingTable> {
        self.module_binding_tables
            .iter()
            .find(|entry| entry.key().1 == profile_id)
            .map(|entry| entry.value().clone())
    }

    /// Require the base DIR artifacts that can contribute module bindings.
    pub(crate) fn require_module_binding_sources(
        &self,
        package_id: PackageId,
        profile_id: ProfileId,
    ) -> ResolveResult<()> {
        let mut collector = BuildRequirementCollector::new();

        // require the full source set before building one transient table
        for module_id in self.module_binding_source_ids(package_id, profile_id)? {
            if let Err(error) = self.require_dir_base(module_id)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                let requirement = error.into_requirement();
                return Err(ResolveError::UnsatisfiedRequirement { requirement });
            }
        }

        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        Ok(())
    }

    /// Build the module binding table for one module and profile.
    pub(crate) fn module_binding_table_for_module(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<ModuleBindingTable> {
        let module = self.program.modules.get(module_id);

        // builtin modules should reuse the active profile binding table when one exists
        if module.is_builtin()
            && let Some(cache) = self.cached_module_binding_table_for_profile(profile_id)
        {
            return Ok(cache);
        }

        self.build_module_binding_table(module.package_id, profile_id)
    }

    /// Resolve a specifier to a module binding target when available.
    pub(crate) fn resolve_module_binding_target(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<ModuleTarget>> {
        let cache = self.module_binding_table_for_module(module_id, profile_id)?;

        if cache.bindings_by_specifier.contains_key(&specifier) {
            return Ok(Some(ModuleTarget::Binding(specifier)));
        }

        Ok(None)
    }

    /// Look up module bindings for a specifier.
    pub(crate) fn module_bindings_for_specifier(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<Vec<ModuleBindingReference>>> {
        let cache = self.module_binding_table_for_module(module_id, profile_id)?;
        Ok(cache.bindings_by_specifier.get(&specifier).cloned())
    }

    /// Detect one runtime module format for a target.
    ///
    /// Returns `None` when the target has no runtime module, or when bindings mix formats.
    pub(crate) fn module_format_for_target(
        &self,
        origin_module_id: ModuleId,
        profile_id: ProfileId,
        target: ModuleTarget,
    ) -> ResolveResult<Option<ModuleFormat>> {
        // module targets expose one direct runtime format
        if let ModuleTarget::Module(module_id) = target {
            let module = self.program.modules.get(module_id);
            let module = module.as_ref();

            // declaration modules do not encode runtime format
            if module.language_type.is_declaration() {
                return Ok(None);
            }

            return Ok(Some(self.program.modules.module_format(module.id)));
        }

        // binding targets may span declarations from multiple modules
        let ModuleTarget::Binding(specifier) = target else {
            return Ok(None);
        };

        let bindings =
            self.module_bindings_for_specifier(origin_module_id, profile_id, specifier)?;
        let Some(bindings) = bindings else {
            return Ok(None);
        };

        // fold runtime formats across binding modules
        let mut saw_commonjs = false;
        let mut saw_esm = false;
        for binding_ref in bindings {
            let module = self.program.modules.get(binding_ref.module_id);
            let module = module.as_ref();

            // declaration modules do not encode runtime format
            if module.language_type.is_declaration() {
                continue;
            }

            if self.program.modules.module_format(module.id).is_commonjs() {
                saw_commonjs = true;
            } else {
                saw_esm = true;
            }

            // mixed runtime formats are not interop-safe
            if saw_commonjs && saw_esm {
                return Ok(None);
            }
        }

        // resolve the folded format
        if saw_commonjs {
            return Ok(Some(ModuleFormat::CommonJs));
        }

        if saw_esm {
            return Ok(Some(ModuleFormat::Esm));
        }

        Ok(None)
    }

    /// Build the module binding table for one package and profile.
    fn build_module_binding_table(
        &self,
        package_id: PackageId,
        profile_id: ProfileId,
    ) -> ResolveResult<ModuleBindingTable> {
        // reuse the table while one compile invocation is in flight
        if let Some(cache) = self.module_binding_tables.get(&(package_id, profile_id)) {
            return Ok(cache.clone());
        }

        // require the full binding source set before reading committed base artifacts
        self.require_module_binding_sources(package_id, profile_id)?;

        let mut cache = ModuleBindingTable::new();

        // collect module bindings from the published base surface
        for module_id in self.module_binding_source_ids(package_id, profile_id)? {
            self.append_module_bindings_from_module(&mut cache, module_id)?;
        }

        // cache the fresh table for later target lookups in this compiler instance
        self.module_binding_tables
            .insert((package_id, profile_id), cache.clone());

        Ok(cache)
    }

    /// Append bindings declared in one module to a module binding table.
    fn append_module_bindings_from_module(
        &self,
        cache: &mut ModuleBindingTable,
        module_id: ModuleId,
    ) -> ResolveResult<()> {
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();

        // only code modules can contribute module bindings
        if !module.is_code() {
            return Ok(());
        }

        // read the already-required base artifact directly
        let dir = self
            .artifact_dir_base(module_id)
            .unwrap_or_else(|| panic!("missing committed base dir artifact for {module_id:?}"));
        let module_bindings = dir.module_bindings.clone();

        for module_binding in module_bindings.iter() {
            let binding_ref = ModuleBindingReference {
                module_id,
                declaration: module_binding.declaration,
            };

            let entries = cache
                .bindings_by_specifier
                .entry(module_binding.specifier)
                .or_default();
            if entries.iter().any(|entry| entry == &binding_ref) {
                continue;
            }
            entries.push(binding_ref);
        }

        Ok(())
    }

    /// Collect ambient modules that can contribute module bindings.
    pub(crate) fn ambient_binding_module_ids(
        &self,
        profile_id: ProfileId,
    ) -> ResolveResult<Vec<ModuleId>> {
        self.ambient_lib_modules_from_input(profile_id)
    }

    /// Collect module ids that belong to one package.
    fn package_module_ids(&self, package_id: PackageId) -> Vec<ModuleId> {
        let mut module_ids = Vec::new();

        // collect modules from the target package
        for module in self.program.modules.iter() {
            if module.package_id == package_id {
                module_ids.push(module.id);
            }
        }

        // keep traversal deterministic
        module_ids.sort_unstable();
        module_ids
    }

    /// Collect the full module-binding source set for one package/profile pair.
    fn module_binding_source_ids(
        &self,
        package_id: PackageId,
        profile_id: ProfileId,
    ) -> ResolveResult<Vec<ModuleId>> {
        let mut module_ids = self.package_module_ids(package_id);
        module_ids.extend(self.ambient_binding_module_ids(profile_id)?);
        module_ids.sort_unstable();
        module_ids.dedup();

        Ok(module_ids)
    }
}
