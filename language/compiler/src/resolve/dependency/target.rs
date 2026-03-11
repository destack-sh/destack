use destack_core::StringId;
use destack_dir::ModuleTarget;
use destack_source::{ModuleId, ModuleVersion, PackageId};
use destack_workspace::{
    ModuleBindingReference, ModuleBindingRegistry, ModuleBindingTable, ModuleBindingTableKey,
    ModuleFormat, ProfileId,
};
use indexmap::IndexMap;

use crate::{Compiler, ResolveResult};

impl Compiler {
    /// Prepare the module binding table for a module and profile.
    pub(crate) fn prepare_module_binding_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<ModuleBindingTableKey> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package_id = module.package_id;
        drop(module);

        // select the module binding cache key
        let (global_key, _) = self.select_global_symbol_table(module_id, profile_id)?;
        let key = ModuleBindingTableKey {
            target_id: global_key.target_id,
            profile_id: global_key.profile_id,
            entry_module: global_key.entry_module,
        };

        // rebuild when module bindings changed since the cache was built
        let mut rebuild_cache = true;
        if let Some(cache) = self.program.index.module_binding_tables.get(&key) {
            rebuild_cache = self.module_binding_table_is_stale(package_id, profile_id, &cache)?;
        }
        if rebuild_cache {
            let cache = self.build_module_binding_table(package_id, profile_id)?;
            self.program
                .index
                .module_binding_tables
                .insert(key.clone(), cache);
        }

        Ok(key)
    }

    /// Resolve a specifier to a module binding target when available.
    pub(crate) fn resolve_module_binding_target(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<ModuleTarget>> {
        let cache_key = self.prepare_module_binding_table(module_id, profile_id)?;
        let Some(cache) = self.program.index.module_binding_tables.get(&cache_key) else {
            return Ok(None);
        };

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
        let cache_key = self.prepare_module_binding_table(module_id, profile_id)?;
        let Some(cache) = self.program.index.module_binding_tables.get(&cache_key) else {
            return Ok(None);
        };
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
            let module = module.read();

            // declaration modules do not encode runtime format
            if module.language_type.is_declaration() {
                return Ok(None);
            }

            return Ok(Some(module.module_format));
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
            let module = module.read();

            // declaration modules do not encode runtime format
            if module.language_type.is_declaration() {
                continue;
            }

            if module.module_format.is_commonjs() {
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
        let mut cache = ModuleBindingTable::new();

        // collect module bindings declared in the current package
        if let Some(registry) = self.program.index.module_binding_registry.get(&package_id) {
            self.append_module_binding_registry(&mut cache, &registry);
        }

        // include ambient lib module bindings visible to this profile
        for module_id in self.ambient_binding_module_ids(profile_id)? {
            self.append_module_bindings_from_module(&mut cache, module_id);
        }

        Ok(cache)
    }

    /// Append one package registry to a module binding table.
    fn append_module_binding_registry(
        &self,
        cache: &mut ModuleBindingTable,
        registry: &ModuleBindingRegistry,
    ) {
        for (module_id, version) in &registry.module_versions {
            cache.module_versions.insert(*module_id, *version);
            cache.registry_module_versions.insert(*module_id, *version);
        }

        for (specifier, bindings) in &registry.bindings_by_specifier {
            let entries = cache.bindings_by_specifier.entry(*specifier).or_default();
            for binding in bindings {
                if entries.iter().any(|entry| entry == binding) {
                    continue;
                }
                entries.push(*binding);
            }
        }
    }

    /// Append bindings declared in one module to a module binding table.
    fn append_module_bindings_from_module(
        &self,
        cache: &mut ModuleBindingTable,
        module_id: ModuleId,
    ) {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        cache.module_versions.insert(module_id, module.version);

        let module_bindings = module.dir_base().module_bindings.read();
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
    }

    /// Collect ambient modules that can contribute module bindings.
    pub(crate) fn ambient_binding_module_ids(
        &self,
        profile_id: ProfileId,
    ) -> ResolveResult<Vec<ModuleId>> {
        self.ambient_lib_modules_from_input(profile_id)
    }

    /// Collect package declared module binding versions for stale checks.
    fn registry_module_binding_versions(
        &self,
        package_id: PackageId,
    ) -> IndexMap<ModuleId, ModuleVersion> {
        let mut versions = IndexMap::new();

        // include package module binding versions
        if let Some(registry) = self.program.index.module_binding_registry.get(&package_id) {
            for (module_id, version) in &registry.module_versions {
                versions.insert(*module_id, *version);
            }
        }

        versions
    }

    /// Return true when module binding inputs changed since caching.
    fn module_binding_table_is_stale(
        &self,
        package_id: PackageId,
        profile_id: ProfileId,
        cache: &ModuleBindingTable,
    ) -> ResolveResult<bool> {
        // compare package registry inputs first
        let expected_registry_versions = self.registry_module_binding_versions(package_id);
        if expected_registry_versions.len() != cache.registry_module_versions.len() {
            return Ok(true);
        }

        for (module_id, expected_version) in expected_registry_versions.clone() {
            let Some(cached_version) = cache.registry_module_versions.get(&module_id) else {
                return Ok(true);
            };
            if cached_version != &expected_version {
                return Ok(true);
            }
        }

        // compare the full module set, including ambient lib module bindings
        let mut expected_module_versions = expected_registry_versions;
        for module_id in self.ambient_binding_module_ids(profile_id)? {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            expected_module_versions.insert(module_id, module.version);
        }

        if expected_module_versions.len() != cache.module_versions.len() {
            return Ok(true);
        }

        for (module_id, expected_version) in expected_module_versions {
            let Some(cached_version) = cache.module_versions.get(&module_id) else {
                return Ok(true);
            };
            if cached_version != &expected_version {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
