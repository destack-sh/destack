use std::collections::{HashSet, VecDeque};

use destack_base::StringId;
use destack_dir::{Declaration, LocalNodeId, ModuleTarget};
use destack_source::{ModuleId, ModuleVersion};
use destack_workspace::{ProfileId, TargetId};
use indexmap::IndexMap;

use crate::{Compiler, ResolveError, ResolveResult, TaskDependencyError};

/// Identify a cached module binding table view for a target and profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ModuleBindingCacheKey {
    /// Target id for the module selection.
    pub target_id: TargetId,
    /// Profile id for the compilation.
    pub profile_id: ProfileId,
    /// Entry module id when target discovery is implicit.
    pub entry_module: Option<ModuleId>,
}

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
pub(crate) struct ModuleBindingCache {
    /// Versions for modules included in the index.
    pub module_versions: IndexMap<ModuleId, ModuleVersion>,
    /// Module bindings by specifier.
    pub bindings_by_specifier: IndexMap<StringId, Vec<ModuleBindingReference>>,
}

impl ModuleBindingCache {
    /// Create an empty table seeded with roots.
    fn new() -> Self {
        Self {
            module_versions: IndexMap::new(),
            bindings_by_specifier: IndexMap::new(),
        }
    }
}

impl Compiler {
    /// Prepare the module binding table for a module and profile.
    pub(super) fn prepare_module_binding_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<ModuleBindingCacheKey> {
        // select the root set for this module
        let (global_key, roots) = self.select_global_symbol_table(module_id, profile_id)?;
        let mut key = ModuleBindingCacheKey {
            target_id: global_key.target_id,
            profile_id: global_key.profile_id,
            entry_module: global_key.entry_module,
        };
        let mut roots = roots;

        // ensure the current module participates in binding discovery
        if !roots.contains(&module_id) {
            roots.push(module_id);
            key.entry_module = Some(module_id);
        }

        // build the cache when missing
        if !self.module_binding_caches.contains_key(&key) {
            let cache = self.build_module_binding_table(&roots)?;
            self.module_binding_caches.insert(key.clone(), cache);
        }

        Ok(key)
    }

    /// Resolve a specifier to a module binding target when available.
    pub(super) fn resolve_module_binding_target(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<ModuleTarget>> {
        let cache_key = self.prepare_module_binding_table(module_id, profile_id)?;
        let Some(cache) = self.module_binding_caches.get(&cache_key) else {
            return Ok(None);
        };
        if cache.bindings_by_specifier.contains_key(&specifier) {
            return Ok(Some(ModuleTarget::Binding(specifier)));
        }
        Ok(None)
    }

    /// Look up module bindings for a specifier.
    pub(super) fn module_bindings_for_specifier(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<Vec<ModuleBindingReference>>> {
        let cache_key = self.prepare_module_binding_table(module_id, profile_id)?;
        let Some(cache) = self.module_binding_caches.get(&cache_key) else {
            return Ok(None);
        };
        Ok(cache.bindings_by_specifier.get(&specifier).cloned())
    }

    /// Build the module binding table for a root module set.
    fn build_module_binding_table(&self, roots: &[ModuleId]) -> ResolveResult<ModuleBindingCache> {
        // initialize the traversal state
        let mut cache = ModuleBindingCache::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.extend(roots.iter().copied());

        // walk the module graph starting from the roots
        while let Some(module_id) = queue.pop_front() {
            if !visited.insert(module_id) {
                continue;
            }

            // ensure bind validation before reading dir data
            self.require_bind_module_validate(module_id)
                .map_err(|error| match error {
                    TaskDependencyError::NotReady { dependency } => {
                        ResolveError::Yield { dependency }
                    }
                    TaskDependencyError::Failed { dependency } => {
                        ResolveError::UnsatisfiedDependency { dependency }
                    }
                })?;

            // load module bindings from the base dir
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let base_dir = module.dir_base();
            cache.module_versions.insert(module_id, module.version);
            for binding in base_dir.module_bindings.read().iter() {
                let binding_ref = ModuleBindingReference {
                    module_id,
                    declaration: binding.declaration,
                };
                cache
                    .bindings_by_specifier
                    .entry(binding.specifier)
                    .or_default()
                    .push(binding_ref);
            }

            // enqueue dependency targets for further discovery
            let tree = base_dir.tree.read();
            let dependency_targets = self.collect_dependency_targets(module_id, &tree);
            for (target, _node) in dependency_targets {
                // skip specifiers that don't resolve to modules
                let remote_module_id =
                    match self.resolve_specifier_to_module(target, Some(module_id)) {
                        Ok(module_id) => module_id,
                        Err(_) => continue,
                    };
                queue.push_back(remote_module_id);
            }
        }

        // include module bindings from any already-bound modules
        for module in self.program.modules.iter() {
            let module = module.read();
            let module_id = module.id;
            if visited.contains(&module_id) {
                continue;
            }
            if module.dir_base_maybe().is_none() {
                continue;
            }

            let base_dir = module.dir_base();
            cache.module_versions.insert(module_id, module.version);
            for binding in base_dir.module_bindings.read().iter() {
                let binding_ref = ModuleBindingReference {
                    module_id,
                    declaration: binding.declaration,
                };
                cache
                    .bindings_by_specifier
                    .entry(binding.specifier)
                    .or_default()
                    .push(binding_ref);
            }
        }

        Ok(cache)
    }
}

#[cfg(test)]
mod tests {
    use crate::TestProgram;
    use destack_dir::StaticKey;

    /// Resolve imports from module declarations.
    #[test]
    fn test_resolve_module_binding_import() {
        // arrange test modules
        let test = TestProgram::memory_sequential();
        let decl_source = r#"
declare module "foo" {
    export const value: number;
}
"#;
        let main_source = r#"
import "./decl.d.ts";
import { value } from "foo";

value;
"#;
        let decl_module_id = test.add_module("decl.d.ts", decl_source);
        let main_module_id = test.add_module("main.ts", main_source);

        // run resolve pipeline
        test.resolve_module(main_module_id);
        test.compile_dump_clean();

        // load symbol data for the main module
        let module = test.program.modules.get(main_module_id);
        let module = module.read();
        let profile = test.default_profile_id(main_module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();

        // locate the imported symbol
        let name_id = test.program.strings.intern("value");
        let scope = symbols.get_scope_by_id(dir.namespace_scope);
        let Some(symbol_id) = scope.find(StaticKey::Name(name_id)) else {
            panic!("expected import binding for value");
        };
        let symbol = symbols.get_symbol(symbol_id);
        let Some(target_symbol) = symbol.target_symbol else {
            panic!("expected import target");
        };

        // assert the import resolves to the module binding declaration
        assert_eq!(target_symbol.module_id, decl_module_id);
    }
}
