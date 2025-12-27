use std::collections::HashSet;

use destack_base::StringId;
use destack_builtin::{
    DOM_CANONICAL_EXPORTS, ES_CANONICAL_EXPORTS, LanguageItem, WORKER_CANONICAL_EXPORTS,
    builtin_lib,
};
use destack_dir::{GlobalSymbolId, StaticKey};
use destack_workspace::ProfileId;
use indexmap::IndexMap;

use crate::{Compiler, ResolveError, ResolveResult, ResolveWarning};

impl Compiler {
    /// Resolve a profile's libraries.
    pub fn resolve_libs(&self, profile_id: ProfileId) -> ResolveResult<()> {
        if !self.options.load_libs {
            return Ok(());
        }
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(());
        };

        self.require_resolve_builtins(profile_id)?;

        // collect libs to resolve
        let profile = self.program.profile(profile_id);
        let mut libs = profile.key.lib.clone();
        if !libs.contains(&"std".to_string()) {
            libs.push("std".to_string());
        }

        // build lib load order with dependencies first
        let ordered_libs = {
            let mut ordered_libs = Vec::new();
            let mut seen_libs = HashSet::new();
            for lib_name in &libs {
                self.collect_lib_dependencies(lib_name, &mut ordered_libs, &mut seen_libs)?;
            }
            ordered_libs
        };

        // load libs in order
        let mut ambient_modules = Vec::new();
        let mut ambient_seen = HashSet::new();
        let mut all_modules = Vec::new();
        let mut all_seen = HashSet::new();
        let mut modules_to_resolve = Vec::new();
        for lib_name in &ordered_libs {
            let lib = builtin_lib(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;
            let module_ids = builtins.load_lib(
                lib_name,
                self.program.files.clone(),
                self.program.modules.clone(),
            );
            let module_ids = module_ids.ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;
            modules_to_resolve.push(module_ids.clone());

            for &module_id in &module_ids {
                if all_seen.insert(module_id) {
                    all_modules.push(module_id);
                }
                if lib.is_ambient && ambient_seen.insert(module_id) {
                    ambient_modules.push(module_id);
                }
            }
        }

        // register ambient module list before resolving symbols
        builtins.set_ambient_libs(profile_id, ambient_modules);

        // resolve modules in order
        for module_ids in modules_to_resolve {
            for module_id in module_ids {
                self.require_resolve_module(module_id, profile_id)?;
            }
        }

        // collect canonical lib symbols
        let mut canonical_name_ids = HashSet::new();
        for name in ES_CANONICAL_EXPORTS
            .iter()
            .chain(DOM_CANONICAL_EXPORTS)
            .chain(WORKER_CANONICAL_EXPORTS)
        {
            let name_id = self.program.strings.intern(name);
            canonical_name_ids.insert(name_id);
        }

        // cache canonical lib symbols for fast lookup
        let mut lib_symbols = IndexMap::new();
        let mut canonical_modules = IndexMap::new();
        for module_id in all_modules {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let symbols = dir.symbols.read();
            let scope = symbols.get_scope_by_id(dir.namespace_scope);

            for (key, symbol_id) in &scope.named_symbols {
                let StaticKey::Name(name_id) = *key else {
                    continue;
                };
                if canonical_name_ids.contains(&name_id) {
                    if let Some(existing_module) = canonical_modules.get(&name_id) {
                        if *existing_module != module_id {
                            self.warning(ResolveWarning::AmbiguousLibSymbol {
                                module: module_id,
                                name: name_id,
                                first_module: *existing_module,
                                second_module: module_id,
                            });
                        }
                    } else {
                        canonical_modules.insert(name_id, module_id);
                    }
                    lib_symbols
                        .entry(name_id)
                        .or_insert(symbol_id.into_global(module_id));
                }
            }
        }

        builtins.set_lib_symbols(profile_id, lib_symbols);

        Ok(())
    }

    /// Collect the dependencies of a lib and add them to the ordered list.
    fn collect_lib_dependencies(
        &self,
        name: &str,
        ordered: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) -> ResolveResult<()> {
        if !seen.insert(name.to_string()) {
            return Ok(());
        }

        // get the lib
        let lib = builtin_lib(name).ok_or_else(|| ResolveError::MissingBuiltinLib {
            name: name.to_string(),
        })?;

        // collect dependencies
        for &dependency in lib.dependencies {
            self.collect_lib_dependencies(dependency, ordered, seen)?;
        }

        ordered.push(name.to_string());
        Ok(())
    }

    /// Resolve all builtin modules and required language items.
    pub fn resolve_builtins(&self, profile: ProfileId) -> ResolveResult<()> {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(());
        };

        // resolve prelude/core modules
        self.require_resolve_module(builtins.prelude_module_id, profile)?;
        for &module_id in builtins.core_module_by_path.values() {
            self.require_resolve_module(module_id, profile)?;
        }

        // resolve all language items
        for item in LanguageItem::all() {
            self.require_language_item(profile, item)?;
        }

        Ok(())
    }

    /// Get a required language item, returning an error if not found.
    pub fn require_language_item(
        &self,
        profile: ProfileId,
        item: LanguageItem,
    ) -> ResolveResult<GlobalSymbolId> {
        // check if builtins are available
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Err(ResolveError::MissingLanguageItem { item });
        };

        // check cache
        if let Some(cached) = builtins.items.get(&item) {
            return Ok(*cached);
        }

        // resolve the module for this language item
        let module_id = builtins.module_for_item(item);
        self.require_resolve_module(module_id, profile)?;

        // resolve the symbol in the module
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();

        // find the symbol in the module's namespace scope
        let name_id = self.program.strings.intern(item.export_name());
        let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
        let key = StaticKey::Name(name_id);
        let Some(symbol_id) = namespace_scope.find(key) else {
            return Err(ResolveError::MissingLanguageItem { item });
        };

        // result
        let global_id = symbol_id.into_global(module_id);
        builtins.items.insert(item, global_id);
        Ok(global_id)
    }

    /// Get a language item from the cache, panicking if not found.
    pub fn expect_language_item(&self, item: LanguageItem) -> GlobalSymbolId {
        let builtins = self
            .program
            .builtins
            .as_ref()
            .expect("builtins not available");
        *builtins
            .items
            .get(&item)
            .unwrap_or_else(|| panic!("language item {item:?} not resolved"))
    }

    /// Get a cached lib symbol for a profile and name.
    pub fn lib_symbol(&self, profile: ProfileId, name: StringId) -> Option<GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.lib_symbol(profile, name)
    }

    /// Get a lib symbol from the cache, panicking if not found.
    pub fn expect_lib_symbol(&self, profile: ProfileId, name: StringId) -> GlobalSymbolId {
        self.lib_symbol(profile, name).unwrap_or_else(|| {
            let name = self.program.strings.get(name);
            panic!("lib symbol '{}' not resolved", name.as_ref())
        })
    }
}

#[cfg(test)]
mod tests {
    use destack_builtin::LanguageItem;

    use crate::{TestProgram, assert_string};

    /// Test that language item modules can be looked up correctly.
    #[test]
    fn test_resolve_language_item_symbol() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.resolve_builtins();
        test.compile();

        let language_item_id = test.compiler.expect_language_item(LanguageItem::Add);
        let language_item_module = test.program.modules.get(language_item_id.module_id);
        let language_item_module = language_item_module.read();
        let profile = test.default_profile_id(language_item_id.module_id);
        let language_item_symbols = language_item_module.dir(profile).symbols.read();
        let language_item_symbol = language_item_symbols.get_symbol(language_item_id.into_local());
        assert_string!(test.program, language_item_symbol.name().unwrap(), "Add");
    }
}
