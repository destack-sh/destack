use std::collections::HashSet;

use destack_base::StringId;
use destack_builtin::{LanguageSymbol, builtin_lib};
use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder, WellKnownSymbol};
use destack_workspace::{Builtins, ProfileId, WellKnownSymbols};
use indexmap::IndexMap;

use crate::resolve::globals::GlobalSymbolCache;
use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};

/// Loaded builtin lib modules collected during resolve.
#[derive(Debug)]
struct LoadedLibModules {
    /// Module batches in lib dependency order.
    modules_to_resolve: Vec<Vec<destack_source::ModuleId>>,
    /// All lib modules in load order without duplicates.
    lib_modules: Vec<destack_source::ModuleId>,
    /// Ambient lib modules in load order without duplicates.
    ambient_modules: Vec<destack_source::ModuleId>,
}

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
        // always include std lib (?)
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
        let loaded_modules = self.load_lib_modules_in_order(builtins, &ordered_libs)?;
        let LoadedLibModules {
            modules_to_resolve,
            lib_modules,
            ambient_modules,
        } = loaded_modules;
        builtins.set_ambient_libs(profile_id, ambient_modules.clone());

        // prepare modules in order
        let mut collector = TaskResultCollector::new();
        for module_ids in &modules_to_resolve {
            for &module_id in module_ids {
                if let Err(error) = self.require_resolve_module_prepare(module_id, profile_id)
                    && let Some(error) = collector.try_collect::<(), _>(Err(error))
                {
                    let dependency = error.into_dependency();
                    return Err(ResolveError::UnsatisfiedDependency { dependency });
                }
            }
        }
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(ResolveError::Yield { dependency });
        }

        // resolve dependency items for lib modules
        let mut collector = TaskResultCollector::new();
        for &module_id in &lib_modules {
            if let Err(error) = self.resolve_dependency_items(module_id, profile_id)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                return Err(error);
            }
        }
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(ResolveError::Yield { dependency });
        }

        // build global symbol cache for lib modules
        let global_cache = self.build_global_symbol_cache_freestanding(&lib_modules, profile_id)?;

        // find declared symbol names from lib definitions
        let declared_names = self.collect_declared_lib_symbol_names(&ordered_libs)?;
        let declared_symbols = self.find_declared_lib_symbols(
            profile_id,
            &lib_modules,
            &global_cache,
            &declared_names,
        );

        // cache declared lib symbols
        builtins.set_declared_lib_symbols(profile_id, declared_symbols.clone());

        // cache well-known symbols
        let well_known_symbols = WellKnownSymbols::build(&self.program.strings, &declared_symbols);
        builtins.set_well_known_symbols(profile_id, well_known_symbols);

        Ok(())
    }

    /// Collect the declared symbol names from builtin libs.
    fn collect_declared_lib_symbol_names(
        &self,
        ordered_libs: &[String],
    ) -> ResolveResult<HashSet<StringId>> {
        // collect declared symbol names from all libs
        let mut declared_name_ids = HashSet::new();
        for lib_name in ordered_libs {
            let lib = builtin_lib(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;
            for &name in lib.declared_symbols {
                declared_name_ids.insert(self.program.strings.intern(name));
            }
        }

        Ok(declared_name_ids)
    }

    /// Find declared lib symbols from the global cache, falling back to exports.
    fn find_declared_lib_symbols(
        &self,
        profile_id: ProfileId,
        lib_modules: &[destack_source::ModuleId],
        global_cache: &GlobalSymbolCache,
        declared_names: &HashSet<StringId>,
    ) -> IndexMap<StringId, GlobalSymbolId> {
        let mut declared_symbols = IndexMap::new();

        for &name_id in declared_names {
            let key = StaticKey::Name(name_id);

            // try global cache first
            if let Some(&symbol_id) = global_cache.symbols.get(&key) {
                declared_symbols.insert(name_id, symbol_id);
                continue;
            }

            // fall back to exports
            for &module_id in lib_modules {
                if declared_symbols.contains_key(&name_id) {
                    break;
                }

                // load the module
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let dir = module.dir(profile_id);
                let exports = dir.exported_symbols.read();
                let tree = dir.tree.read();

                // resolve the symbol from the module's export table
                if let Some(symbol_id) = self.resolve_exported_symbol(
                    module_id,
                    &exports,
                    &tree,
                    SymbolSpaceOrder::ValueThenType,
                    key,
                ) {
                    declared_symbols.insert(name_id, symbol_id);
                }
            }
        }

        declared_symbols
    }

    /// Load lib modules in dependency order and collect module lists.
    fn load_lib_modules_in_order(
        &self,
        builtins: &Builtins,
        ordered_libs: &[String],
    ) -> ResolveResult<LoadedLibModules> {
        let mut ambient_modules = Vec::new();
        let mut ambient_seen = HashSet::new();
        let mut all_modules = Vec::new();
        let mut all_seen = HashSet::new();
        let mut modules_to_resolve = Vec::new();

        for lib_name in ordered_libs {
            // load the lib definition
            let lib = builtin_lib(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;

            // load the lib modules
            let module_ids = builtins.load_lib(
                lib_name,
                self.program.files.clone(),
                self.program.modules.clone(),
            );
            let module_ids = module_ids.ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;
            modules_to_resolve.push(module_ids.clone());

            // collect into the appropriate lists
            for &module_id in &module_ids {
                if all_seen.insert(module_id) {
                    all_modules.push(module_id);
                }
                if lib.is_ambient && ambient_seen.insert(module_id) {
                    ambient_modules.push(module_id);
                }
            }
        }

        Ok(LoadedLibModules {
            modules_to_resolve,
            lib_modules: all_modules,
            ambient_modules,
        })
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
        for item in LanguageSymbol::all() {
            self.require_language_symbol(profile, item)?;
        }

        Ok(())
    }

    /// Get a required language item, returning an error if not found.
    pub fn require_language_symbol(
        &self,
        profile: ProfileId,
        item: LanguageSymbol,
    ) -> ResolveResult<GlobalSymbolId> {
        // check if builtins are available
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Err(ResolveError::MissingLanguageSymbol { item });
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

        // find the symbol in the module's export table
        let name_id = self.program.strings.intern(item.export_name());
        let key = StaticKey::Name(name_id);
        let exports = dir.exported_symbols.read();
        let tree = dir.tree.read();
        let export_spaces = SymbolSpaceOrder::ValueThenType;
        let Some(symbol_id) =
            self.resolve_exported_symbol(module_id, &exports, &tree, export_spaces, key)
        else {
            return Err(ResolveError::MissingLanguageSymbol { item });
        };

        // result
        builtins.items.insert(item, symbol_id);
        Ok(symbol_id)
    }

    /// Get a language item from the cache, returning None if not found.
    pub fn get_language_symbol(&self, item: LanguageSymbol) -> Option<GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.items.get(&item).map(|r| *r)
    }

    /// Get a language item from the cache, panicking if not found.
    pub fn language_symbol(&self, item: LanguageSymbol) -> GlobalSymbolId {
        self.get_language_symbol(item)
            .unwrap_or_else(|| panic!("language item {item:?} not available"))
    }

    /// Get a cached declared lib symbol for a profile and name.
    pub fn get_declared_lib_symbol(
        &self,
        profile: ProfileId,
        name: StringId,
    ) -> Option<GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.get_declared_lib_symbol(profile, name)
    }

    /// Get a declared lib symbol from the cache, panicking if not found.
    pub fn declared_lib_symbol(&self, profile: ProfileId, name: StringId) -> GlobalSymbolId {
        self.get_declared_lib_symbol(profile, name)
            .unwrap_or_else(|| {
                let name = self.program.strings.get(name);
                panic!("declared lib symbol '{}' not available", name.as_ref())
            })
    }

    /// Get well-known symbols for a profile.
    pub fn get_well_known_symbols(&self, profile: ProfileId) -> Option<WellKnownSymbols> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.well_known_symbols(profile)
    }

    /// Get well-known symbols for a profile, panicking if not found.
    pub fn well_known_symbols(&self, profile: ProfileId) -> WellKnownSymbols {
        self.get_well_known_symbols(profile)
            .unwrap_or_else(|| panic!("well-known symbols not available for profile {profile:?}"))
    }

    /// Get a specific well-known symbol for a profile.
    pub fn get_well_known_symbol(
        &self,
        profile: ProfileId,
        symbol: WellKnownSymbol,
    ) -> Option<GlobalSymbolId> {
        let well_known_symbols = self.get_well_known_symbols(profile)?;
        well_known_symbols.get_symbol(symbol)
    }

    /// Get a specific well-known symbol from the type space when available.
    pub fn get_well_known_type_symbol(
        &self,
        profile: ProfileId,
        symbol: WellKnownSymbol,
    ) -> Option<GlobalSymbolId> {
        let symbol_id = self.get_well_known_symbol(profile, symbol)?;
        Some(self.resolve_well_known_type_symbol(profile, symbol_id))
    }

    /// Get a specific well-known symbol for a profile, panicking if not found.
    pub fn well_known_symbol(&self, profile: ProfileId, symbol: WellKnownSymbol) -> GlobalSymbolId {
        self.get_well_known_symbol(profile, symbol)
            .unwrap_or_else(|| {
                panic!("well-known symbol {symbol:?} not available for profile {profile:?}")
            })
    }

    /// Find the type-space counterpart of a symbol if one exists.
    /// FUGU #Cleanup: remove resolve_well_known_type_symbol in favor of symbol-space-keyed well known symbols
    fn resolve_well_known_type_symbol(
        &self,
        profile: ProfileId,
        symbol_id: GlobalSymbolId,
    ) -> GlobalSymbolId {
        let module = self.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(profile) else {
            return symbol_id;
        };
        let symbols = dir.symbols.read();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        // already in type space
        if matches!(symbol.space, SymbolSpace::Type | SymbolSpace::TypeValue) {
            return symbol_id;
        }

        let Some(name) = symbol.name() else {
            return symbol_id;
        };

        // scan the namespace scope for a type-space symbol with the same name
        let scope = symbols.get_scope_by_id(dir.namespace_scope);
        let mut type_fallback = None;
        for (key, candidate_id) in scope.named_symbols.iter().rev() {
            let StaticKey::Name(candidate_name) = *key else {
                continue;
            };
            if candidate_name != name {
                continue;
            }

            let candidate = symbols.get_symbol(*candidate_id);
            match candidate.space {
                // prefer TypeValue (both type and value)
                SymbolSpace::TypeValue => return candidate_id.into_global(module.id),
                // fall back to Type
                SymbolSpace::Type => {
                    if type_fallback.is_none() {
                        type_fallback = Some(*candidate_id);
                    }
                }
                _ => {}
            }
        }

        type_fallback
            .map(|id| id.into_global(module.id))
            .unwrap_or(symbol_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_builtin::{LIBS, LanguageSymbol, STD_LIB};
    use destack_dir::{WellKnownSymbol, WellKnownSymbolKey};

    use crate::{TaskPhase, TestProgram, assert_string};

    /// Test that language item modules can be looked up correctly.
    #[test]
    fn test_resolve_language_symbol() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.resolve_builtins();
        test.compile();

        let symbol_id = test.compiler.language_symbol(LanguageSymbol::Add);
        let module = test.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let profile = test.default_profile_id(symbol_id.module_id);
        let symbols = module.dir(profile).symbols.read();
        let symbol = symbols.get_symbol(symbol_id.into_local());
        assert_string!(test.program, symbol.name().unwrap(), "Add");
    }

    /// Resolve declared lib symbols for all builtin libs.
    #[test]
    fn test_resolve_builtin_lib_symbols() {
        for lib in LIBS {
            let test = TestProgram::memory_sequential_with_prelude_and_libs()
                .with_profile_libs(&[lib.name]);
            test.resolve_builtins();
            test.resolve_libs();
            test.compile();
            test.check_no_diagnostics_up_to_including_phase(TaskPhase::Resolve);

            let profile = test.default_profile_id_for_root();
            for &symbol in lib.declared_symbols {
                let name_id = test.program.strings.intern(symbol);
                let declared_symbol = test.compiler.get_declared_lib_symbol(profile, name_id);
                assert!(
                    declared_symbol.is_some(),
                    "missing declared lib symbol {}:{}",
                    lib.name,
                    symbol
                );
            }
        }
    }

    /// Resolve well known symbols from builtin libs.
    #[test]
    fn test_resolve_well_known_symbols() {
        let test =
            TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es2020"]);
        test.resolve_builtins();
        test.resolve_libs();
        test.compile();

        let profile = test.default_profile_id_for_root();
        let well_known = test
            .compiler
            .get_well_known_symbols(profile)
            .unwrap_or_else(|| panic!("missing well known symbols for test profile"));
        for symbol in WellKnownSymbol::all() {
            assert!(
                well_known.get_symbol(symbol).is_some(),
                "missing well-known symbol {symbol:?}"
            );
        }

        for symbol in WellKnownSymbolKey::all() {
            let key = well_known
                .get_key(symbol)
                .unwrap_or_else(|| panic!("missing well-known key {symbol:?}"));
            assert_string!(test.program, key.member, symbol.member_name());
            assert_string!(test.program, key.global_name, symbol.global_symbol_name());
        }
    }

    /// Resolve all builtin libs (without errors).
    #[test]
    fn test_resolve_all_builtin_libs() {
        // collect all lib names
        let lib_names: Vec<&str> = std::iter::once(&STD_LIB)
            .chain(LIBS.iter())
            .map(|lib| lib.name)
            .collect();

        // create single TestProgram with all libs and resolve once
        let test =
            TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&lib_names);
        test.resolve_builtins();
        test.resolve_libs();
        test.compile();
    }

    /// Analyze all builtin libs (without errors).
    #[test]
    #[ignore] // FUGU: compile_check_clean on test_analyze_all_builtin_libs (again..)
    fn test_analyze_all_builtin_libs() {
        // collect all libs into one test program
        let lib_names: Vec<&str> = std::iter::once(&STD_LIB)
            .chain(LIBS.iter())
            .map(|lib| lib.name)
            .collect();
        let test =
            TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&lib_names);

        // resolve once
        test.resolve_builtins();
        test.resolve_libs();
        test.compile_check_clean();

        // analyze each module from each lib
        let builtins = test.program.builtins.as_ref().unwrap();
        for lib in std::iter::once(&STD_LIB).chain(LIBS.iter()) {
            let lib_modules = builtins
                .load_lib(
                    lib.name,
                    test.program.files.clone(),
                    test.program.modules.clone(),
                )
                .unwrap();
            for module_id in lib_modules {
                test.analyze_module(module_id);
            }
        }
        test.compile_check_clean();
    }
}
