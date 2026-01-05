use std::collections::HashSet;

use destack_base::StringId;
use destack_builtin::{LanguageItem, builtin_lib};
use destack_dir::{
    DependencyItem, Export, ExportSpaceOrder, GlobalSymbolId, NodeTree, StaticKey, SymbolSpace,
    WellKnownSymbol,
};
use destack_workspace::{ProfileId, WellKnownSymbols};
use indexmap::IndexMap;

use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};

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

        // yield when any prepare tasks are still pending
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(ResolveError::Yield { dependency });
        }

        // collect and cache canonical lib symbols
        self.cache_canonical_lib_symbols(builtins, profile_id, &ordered_libs, all_modules)
    }

    /// Collect canonical exports from libs and cache them.
    fn cache_canonical_lib_symbols(
        &self,
        builtins: &destack_workspace::LanguageBuiltins,
        profile_id: ProfileId,
        ordered_libs: &[String],
        all_modules: Vec<destack_source::ModuleId>,
    ) -> ResolveResult<()> {
        // collect canonical export names from all libs
        let mut canonical_name_ids = HashSet::new();
        for lib_name in ordered_libs {
            let lib = builtin_lib(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;
            for &name in lib.canonical_exports {
                canonical_name_ids.insert(self.program.strings.intern(name));
            }
        }

        // match exports across modules in lib precedence order
        let mut lib_symbols = IndexMap::new();
        let mut pending_dependencies = HashSet::new();
        let export_spaces = ExportSpaceOrder::ValueThenType;

        for module_id in all_modules {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let exports = dir.exported_symbols.read();
            let tree = dir.tree.read();

            for &name_id in &canonical_name_ids {
                if lib_symbols.contains_key(&name_id) {
                    continue;
                }
                let key = StaticKey::Name(name_id);

                // defer when reexport target is unresolved
                if self.has_unresolved_reexport(&exports, &tree, export_spaces, key) {
                    pending_dependencies.insert(module_id);
                    continue;
                }

                if let Some(symbol_id) =
                    self.resolve_exported_symbol(module_id, &exports, &tree, export_spaces, key)
                {
                    lib_symbols.insert(name_id, symbol_id);
                }
            }
        }

        // resolve dependency items needed for canonical exports
        if !pending_dependencies.is_empty() {
            let mut collector = TaskResultCollector::new();
            for module_id in pending_dependencies {
                if let Err(error) = self.resolve_dependency_items(module_id, profile_id)
                    && let Some(error) = collector.try_collect::<(), _>(Err(error))
                {
                    return Err(error);
                }
            }
            if let Some(dependency) = collector.try_into_yield_all() {
                return Err(ResolveError::Yield { dependency });
            }
        }

        // cache lib symbols and well-known symbols
        builtins.set_lib_symbols(profile_id, lib_symbols.clone());
        let well_known_symbols = WellKnownSymbols::build(&self.program.strings, &lib_symbols);
        builtins.set_well_known_symbols(profile_id, well_known_symbols);

        Ok(())
    }

    /// Check if an export key has an unresolved reexport target.
    fn has_unresolved_reexport(
        &self,
        exports: &IndexMap<(SymbolSpace, StaticKey), Export>,
        tree: &NodeTree,
        export_spaces: ExportSpaceOrder,
        key: StaticKey,
    ) -> bool {
        export_spaces.spaces().iter().any(|space| {
            let Some(export) = exports.get(&(*space, key)) else {
                return false;
            };
            if export.kind != destack_dir::ExportKind::ReExport {
                return false;
            }
            let Some(item) = export.item else {
                return false;
            };
            tree.get::<DependencyItem>(item).target_symbol().is_none()
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

        // find the symbol in the module's export table
        let name_id = self.program.strings.intern(item.export_name());
        let key = StaticKey::Name(name_id);
        let exports = dir.exported_symbols.read();
        let tree = dir.tree.read();
        let export_spaces = ExportSpaceOrder::ValueThenType;
        let Some(symbol_id) =
            self.resolve_exported_symbol(module_id, &exports, &tree, export_spaces, key)
        else {
            return Err(ResolveError::MissingLanguageItem { item });
        };

        // result
        builtins.items.insert(item, symbol_id);
        Ok(symbol_id)
    }

    /// Get a language item from the cache, returning None if not found.
    pub fn get_language_item(&self, item: LanguageItem) -> Option<GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.items.get(&item).map(|r| *r)
    }

    /// Get a language item from the cache, panicking if not found.
    pub fn language_item(&self, item: LanguageItem) -> GlobalSymbolId {
        self.get_language_item(item)
            .unwrap_or_else(|| panic!("language item {item:?} not available"))
    }

    /// Get a cached lib symbol for a profile and name.
    pub fn get_lib_item(&self, profile: ProfileId, name: StringId) -> Option<GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.lib_symbol(profile, name)
    }

    /// Get a lib symbol from the cache, panicking if not found.
    pub fn lib_item(&self, profile: ProfileId, name: StringId) -> GlobalSymbolId {
        self.get_lib_item(profile, name).unwrap_or_else(|| {
            let name = self.program.strings.get(name);
            panic!("lib symbol '{}' not available", name.as_ref())
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
        Some(self.prefer_type_symbol_for_name(profile, symbol_id))
    }

    /// Get a specific well-known symbol for a profile, panicking if not found.
    pub fn well_known_symbol(&self, profile: ProfileId, symbol: WellKnownSymbol) -> GlobalSymbolId {
        self.get_well_known_symbol(profile, symbol)
            .unwrap_or_else(|| {
                panic!("well-known symbol {symbol:?} not available for profile {profile:?}")
            })
    }

    /// Prefer a type-space symbol for the given name when one exists.
    fn prefer_type_symbol_for_name(
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
        if matches!(symbol.space, SymbolSpace::Type | SymbolSpace::TypeValue) {
            return symbol_id;
        }

        let Some(name) = symbol.name() else {
            return symbol_id;
        };

        let scope = symbols.get_scope_by_id(dir.namespace_scope);
        let mut fallback = None;
        for (key, candidate_id) in scope.named_symbols.iter().rev() {
            let StaticKey::Name(candidate_name) = *key else {
                continue;
            };
            if candidate_name != name {
                continue;
            }
            let candidate = symbols.get_symbol(*candidate_id);
            match candidate.space {
                SymbolSpace::TypeValue => return candidate_id.into_global(module.id),
                SymbolSpace::Type => {
                    if fallback.is_none() {
                        fallback = Some(*candidate_id);
                    }
                }
                _ => {}
            }
        }

        fallback
            .map(|candidate_id| candidate_id.into_global(module.id))
            .unwrap_or(symbol_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_builtin::{LIBS, LanguageItem, STD_LIB};
    use destack_dir::{WellKnownSymbol, WellKnownSymbolKey};

    use crate::{TestProgram, assert_string};

    /// Test that language item modules can be looked up correctly.
    #[test]
    fn test_resolve_language_item_symbol() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.resolve_builtins();
        test.compile();

        let language_item_id = test.compiler.language_item(LanguageItem::Add);
        let language_item_module = test.program.modules.get(language_item_id.module_id);
        let language_item_module = language_item_module.read();
        let profile = test.default_profile_id(language_item_id.module_id);
        let language_item_symbols = language_item_module.dir(profile).symbols.read();
        let language_item_symbol = language_item_symbols.get_symbol(language_item_id.into_local());
        assert_string!(test.program, language_item_symbol.name().unwrap(), "Add");
    }

    /// Test that builtin lib symbols can be resolved and cached.
    #[test]
    fn test_resolve_builtin_lib_symbol() {
        let test =
            TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es5"]);
        test.resolve_builtins();
        test.resolve_libs();
        test.compile();

        let profile = test.default_profile_id_for_root();
        let array_name = test.program.strings.intern("Array");

        assert!(test.compiler.get_lib_item(profile, array_name).is_some());
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
    #[ignore] // FUGU: compile_check_clean on test_analyze_all_builtin_libs
    fn test_analyze_all_builtin_libs() {
        // collect all lib names
        let lib_names: Vec<&str> = std::iter::once(&STD_LIB)
            .chain(LIBS.iter())
            .map(|lib| lib.name)
            .collect();

        // create single TestProgram with all libs
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
    }
}
