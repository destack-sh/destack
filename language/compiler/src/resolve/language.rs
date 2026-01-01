use std::collections::HashSet;

use destack_base::StringId;
use destack_builtin::{LanguageItem, WellKnownSymbol, builtin_lib};
use destack_dir::{DependencyItem, Expression, GlobalSymbolId, NodeTree, StaticKey};
use destack_workspace::{ProfileId, WellKnownSymbols};
use indexmap::IndexMap;

use crate::{Compiler, ResolveError, ResolveResult};

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
        for module_ids in &modules_to_resolve {
            for &module_id in module_ids {
                self.require_resolve_module_prepare(module_id, profile_id)?;
            }
        }

        // resolve lib module symbols for ambient lookups when imports are present
        let mut seen_modules = HashSet::new();
        for module_ids in &modules_to_resolve {
            for &module_id in module_ids {
                if !seen_modules.insert(module_id) {
                    continue;
                }
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let dir = module.dir(profile_id);
                let tree = dir.tree.read();
                if self.module_requires_direct_resolve(&tree) {
                    self.require_resolve_module_direct(module_id, profile_id)?;
                }
            }
        }

        // collect canonical lib symbols
        let mut canonical_name_ids = HashSet::new();
        for lib_name in &ordered_libs {
            let lib = builtin_lib(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;
            for &name in lib.canonical_exports {
                let name_id = self.program.strings.intern(name);
                canonical_name_ids.insert(name_id);
            }
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

            // prefer the last symbol so lib symbols match ambient resolution order
            for (key, symbol_id) in scope.named_symbols.iter().rev() {
                let StaticKey::Name(name_id) = *key else {
                    continue;
                };
                if canonical_name_ids.contains(&name_id) {
                    canonical_modules.insert(name_id, module_id);
                    lib_symbols
                        .entry(name_id)
                        .or_insert(symbol_id.into_global(module_id));
                }
            }
        }

        builtins.set_lib_symbols(profile_id, lib_symbols.clone());
        let well_known_symbols = WellKnownSymbols::build(&self.program.strings, &lib_symbols);
        builtins.set_well_known_symbols(profile_id, well_known_symbols);

        Ok(())
    }

    /// Whether a module needs direct resolution to handle dependencies.
    fn module_requires_direct_resolve(&self, tree: &NodeTree) -> bool {
        // check unresolved imports or reexports
        for expression_id in tree.iter_node_ids_of_type::<Expression>() {
            match tree.get(expression_id) {
                Expression::UnresolvedImport { .. } | Expression::UnresolvedReExport { .. } => {
                    return true;
                }
                _ => {}
            }
        }

        // check unresolved dependency items
        for item_id in tree.iter_node_ids_of_type::<DependencyItem>() {
            match tree.get(item_id) {
                DependencyItem::UnresolvedRemote { .. }
                | DependencyItem::UnresolvedLocal { .. } => return true,
                _ => {}
            }
        }

        false
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

    /// Get a specific well-known symbol for a profile, panicking if not found.
    pub fn well_known_symbol(&self, profile: ProfileId, symbol: WellKnownSymbol) -> GlobalSymbolId {
        self.get_well_known_symbol(profile, symbol)
            .unwrap_or_else(|| {
                panic!("well-known symbol {symbol:?} not available for profile {profile:?}")
            })
    }
}

#[cfg(test)]
mod tests {
    use destack_builtin::{LIBS, LanguageItem, STD_LIB, WellKnownSymbol};

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
            TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es2018"]);
        test.resolve_builtins();
        test.resolve_libs();
        test.compile();

        let profile = test.default_profile_id_for_root();
        let well_known = test
            .compiler
            .get_well_known_symbols(profile)
            .unwrap_or_else(|| panic!("missing well known symbols for test profile"));
        for symbol in WellKnownSymbol::all() {
            if symbol.export_name().is_some() {
                assert!(
                    well_known.get_symbol(symbol).is_some(),
                    "missing well-known symbol {symbol:?}"
                );
            }

            if let Some(expected_member) = symbol.member_name() {
                let key = well_known
                    .get_key(symbol)
                    .unwrap_or_else(|| panic!("missing well-known key {symbol:?}"));
                assert_string!(test.program, key.member, expected_member);
            }

            if let Some(expected_name) = symbol.global_symbol_name() {
                let key = well_known
                    .get_key(symbol)
                    .unwrap_or_else(|| panic!("missing well-known key {symbol:?}"));
                assert_string!(test.program, key.global_name, expected_name);
            }
        }
    }

    /// Resolve all builtin libs without errors.
    #[test]
    fn test_resolve_all_builtin_libs() {
        for lib in std::iter::once(&STD_LIB).chain(LIBS.iter()) {
            let test = TestProgram::memory_sequential_with_prelude_and_libs()
                .with_profile_libs(&[lib.name]);
            test.resolve_builtins();
            test.resolve_libs();
            test.compile();
        }
    }
}
