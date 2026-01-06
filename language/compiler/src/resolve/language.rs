use std::collections::HashSet;

use destack_base::StringId;
use destack_builtin::{LanguageSymbol, builtin_lib};
use destack_dir::{
    GlobalSymbolId, LocalScopeId, LocalScopeMark, LocalSymbolId, ModuleBinding, StaticKey,
    SymbolBinding, SymbolKind, SymbolSpace, SymbolSpaceOrder, SymbolTable, WellKnownSymbol,
};
use destack_workspace::{Builtins, ProfileId, WellKnownSymbols};
use indexmap::IndexMap;

use crate::{Compiler, ResolveError, ResolveResult, TaskResultCollector};

/// Loaded builtin lib modules collected during resolve.
#[derive(Debug)]
struct LoadedLibModules {
    /// Module batches in lib dependency order.
    modules_to_resolve: Vec<Vec<destack_source::ModuleId>>,
    /// All lib modules in load order without duplicates.
    all_modules: Vec<destack_source::ModuleId>,
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
            all_modules,
            ambient_modules,
        } = loaded_modules;

        // register ambient module list before resolving symbols
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

        // yield when any prepare tasks are still pending
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(ResolveError::Yield { dependency });
        }

        // resolve dependency items for lib modules
        let mut collector = TaskResultCollector::new();
        for &module_id in &all_modules {
            if let Err(error) = self.resolve_dependency_items(module_id, profile_id)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                return Err(error);
            }
        }
        if let Some(dependency) = collector.try_into_yield_all() {
            return Err(ResolveError::Yield { dependency });
        }

        // collect declared symbol names from lib definitions
        let declared_names = self.collect_declared_lib_symbol_names(&ordered_libs)?;

        // collect declared symbols from exports
        let mut declared_symbols =
            self.find_declared_lib_symbols_in_exports(profile_id, &all_modules, &declared_names)?;

        // fill remaining symbols from ambient globals
        let remaining_names: HashSet<StringId> = declared_names
            .iter()
            .copied()
            .filter(|name_id| !declared_symbols.contains_key(name_id))
            .collect();
        if !remaining_names.is_empty() {
            let global_symbols = self.find_declared_lib_symbols_in_global_scopes(
                profile_id,
                &ambient_modules,
                &remaining_names,
            )?;
            for (name, symbol_id) in global_symbols {
                declared_symbols.entry(name).or_insert(symbol_id);
            }
        }

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

        Ok(LoadedLibModules {
            modules_to_resolve,
            all_modules,
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
        Some(self.prefer_type_symbol_for_name(profile, symbol_id))
    }

    /// Get a specific well-known symbol for a profile, panicking if not found.
    pub fn well_known_symbol(&self, profile: ProfileId, symbol: WellKnownSymbol) -> GlobalSymbolId {
        self.get_well_known_symbol(profile, symbol)
            .unwrap_or_else(|| {
                panic!("well-known symbol {symbol:?} not available for profile {profile:?}")
            })
    }

    /// Collect declared lib symbols from export tables.
    fn find_declared_lib_symbols_in_exports(
        &self,
        profile_id: ProfileId,
        modules: &[destack_source::ModuleId],
        declared_names: &HashSet<StringId>,
    ) -> ResolveResult<IndexMap<StringId, GlobalSymbolId>> {
        // match exports across modules in lib precedence order
        let mut lib_symbols = IndexMap::new();
        let export_spaces = SymbolSpaceOrder::ValueThenType;
        for &module_id in modules {
            if lib_symbols.len() >= declared_names.len() {
                break;
            }

            // read module symbols
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let exports = dir.exported_symbols.read();
            let tree = dir.tree.read();

            // resolve declared names from the module export table
            for &name_id in declared_names {
                if lib_symbols.contains_key(&name_id) {
                    continue;
                }
                let key = StaticKey::Name(name_id);
                if let Some(symbol_id) =
                    self.resolve_exported_symbol(module_id, &exports, &tree, export_spaces, key)
                {
                    lib_symbols.insert(name_id, symbol_id);
                }
            }
        }

        Ok(lib_symbols)
    }

    /// Collect declared lib symbols from ambient globals.
    fn find_declared_lib_symbols_in_global_scopes(
        &self,
        profile_id: ProfileId,
        ambient_modules: &[destack_source::ModuleId],
        declared_names: &HashSet<StringId>,
    ) -> ResolveResult<IndexMap<StringId, GlobalSymbolId>> {
        // collect ambient global symbols in lib precedence order
        let mut ambient_symbols: IndexMap<StringId, GlobalSymbolId> = IndexMap::new();
        let space_order = SymbolSpaceOrder::ValueThenType;
        let global_name = self.program.strings.intern("global");

        // scan ambient modules in load order
        for &module_id in ambient_modules {
            if declared_names
                .iter()
                .all(|name_id| ambient_symbols.contains_key(name_id))
            {
                break;
            }

            // read module symbols
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let symbols = dir.symbols.read();

            // resolve declared names from the global scope chain
            let resolved_symbols = Self::find_declared_lib_symbols_in_scope_chain(
                &symbols,
                dir.global_augmentation_scope,
                space_order,
                declared_names,
            );

            // add the module symbols to the ambient cache when missing
            for (name, symbol_id) in resolved_symbols {
                if ambient_symbols.contains_key(&name) {
                    continue;
                }
                ambient_symbols.insert(name, symbol_id.into_global(module_id));
            }

            // resolve declared names from module binding global namespaces
            let module_bindings = dir.module_bindings.read();
            let binding_symbols = Self::find_declared_lib_symbols_in_module_bindings(
                &symbols,
                &module_bindings,
                global_name,
                space_order,
                declared_names,
            );

            // add the module binding symbols to the ambient cache when missing
            for (name, symbol_id) in binding_symbols {
                if ambient_symbols.contains_key(&name) {
                    continue;
                }
                ambient_symbols.insert(name, symbol_id.into_global(module_id));
            }

            // resolve remaining names from the module symbol table
            let remaining_names: HashSet<StringId> = declared_names
                .iter()
                .copied()
                .filter(|name_id| !ambient_symbols.contains_key(name_id))
                .collect();
            if remaining_names.is_empty() {
                continue;
            }

            let fallback_symbols = Self::find_declared_lib_symbols_in_symbol_table(
                &symbols,
                space_order,
                &remaining_names,
            );

            // add the fallback symbols to the ambient cache when missing
            for (name, symbol_id) in fallback_symbols {
                if ambient_symbols.contains_key(&name) {
                    continue;
                }
                ambient_symbols.insert(name, symbol_id.into_global(module_id));
            }
        }

        Ok(ambient_symbols)
    }

    /// Collect declared lib symbols from module binding scopes.
    fn find_declared_lib_symbols_in_module_bindings(
        symbols: &SymbolTable,
        module_bindings: &[ModuleBinding],
        global_name: StringId,
        space_order: SymbolSpaceOrder,
        declared_names: &HashSet<StringId>,
    ) -> IndexMap<StringId, LocalSymbolId> {
        // collect matching symbols across module bindings
        let mut binding_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();

        // scan each module binding scope
        for binding in module_bindings {
            let binding_scope = symbols.get_scope_by_id(binding.scope);

            // find global namespaces in the binding scope
            for (key, symbol_id) in &binding_scope.named_symbols {
                let StaticKey::Name(name) = *key else {
                    continue;
                };
                if name != global_name {
                    continue;
                }

                let symbol = symbols.get_symbol(*symbol_id);
                if symbol.kind != SymbolKind::Namespace {
                    continue;
                }

                // resolve declared names from the global namespace scope
                let resolved_symbols = Self::find_declared_lib_symbols_in_scope_chain(
                    symbols,
                    symbol.scope.0,
                    space_order,
                    declared_names,
                );

                // keep the first symbol found for each name
                for (name, symbol_id) in resolved_symbols {
                    binding_symbols.entry(name).or_insert(symbol_id);
                }
            }

            // resolve declared names from the module binding scope
            let resolved_symbols = Self::find_declared_lib_symbols_in_scope_chain(
                symbols,
                binding.scope,
                space_order,
                declared_names,
            );

            // keep the first symbol found for each name
            for (name, symbol_id) in resolved_symbols {
                binding_symbols.entry(name).or_insert(symbol_id);
            }
        }

        binding_symbols
    }

    /// Collect declared lib symbols from the full symbol table.
    fn find_declared_lib_symbols_in_symbol_table(
        symbols: &SymbolTable,
        space_order: SymbolSpaceOrder,
        declared_names: &HashSet<StringId>,
    ) -> IndexMap<StringId, LocalSymbolId> {
        // collect candidate symbols by space
        let mut fallback_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
        let mut value_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
        let mut type_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
        let mut type_value_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();

        // scan all symbols in the table
        for symbol_index in 0..symbols.symbol_count() {
            let symbol_id = LocalSymbolId::new(symbol_index);
            let symbol = symbols.get_symbol(symbol_id);
            let Some(StaticKey::Name(name)) = symbol.key else {
                continue;
            };
            if !declared_names.contains(&name) {
                continue;
            }
            if symbol.binding != SymbolBinding::Ambient {
                continue;
            }

            fallback_symbols.entry(name).or_insert(symbol_id);

            match symbol.space {
                SymbolSpace::Value => {
                    value_symbols.entry(name).or_insert(symbol_id);
                }
                SymbolSpace::Type => {
                    type_symbols.entry(name).or_insert(symbol_id);
                }
                SymbolSpace::TypeValue => {
                    type_value_symbols.entry(name).or_insert(symbol_id);
                }
                SymbolSpace::Label => {}
            }
        }

        // select preferred symbols by space order
        let mut resolved_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
        for (name, fallback_id) in fallback_symbols {
            let preferred = space_order.spaces().iter().find_map(|space| match space {
                SymbolSpace::Type => type_symbols
                    .get(&name)
                    .copied()
                    .or(type_value_symbols.get(&name).copied()),
                SymbolSpace::Value => value_symbols
                    .get(&name)
                    .copied()
                    .or(type_value_symbols.get(&name).copied()),
                SymbolSpace::TypeValue => type_value_symbols.get(&name).copied(),
                SymbolSpace::Label => None,
            });

            let symbol_id = preferred.unwrap_or(fallback_id);
            resolved_symbols.insert(name, symbol_id);
        }

        resolved_symbols
    }

    /// Collect declared lib symbols from a scope chain.
    fn find_declared_lib_symbols_in_scope_chain(
        symbols: &SymbolTable,
        scope_id: LocalScopeId,
        space_order: SymbolSpaceOrder,
        declared_names: &HashSet<StringId>,
    ) -> IndexMap<StringId, LocalSymbolId> {
        // track preferred and fallback symbols in the scope chain
        let mut resolved_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
        let mut fallback_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();

        // walk the scope chain from inner to outer
        let mut scope_id = scope_id;
        let mut scope_mark = LocalScopeMark::end();
        loop {
            // read scope and clamp to the active mark
            let scope = symbols.get_scope_by_id(scope_id);
            let limit = (scope_mark.0 as usize).min(scope.named_symbols.len());

            // collect nearest symbols per space in this scope
            let mut value_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
            let mut type_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
            let mut type_value_symbols: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
            let mut scope_fallback: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
            for (key, symbol_id) in scope.named_symbols.iter().take(limit).rev() {
                let StaticKey::Name(name) = *key else {
                    continue;
                };
                if !declared_names.contains(&name) {
                    continue;
                }

                if !scope_fallback.contains_key(&name) {
                    scope_fallback.insert(name, *symbol_id);
                }

                let symbol = symbols.get_symbol(*symbol_id);
                match symbol.space {
                    SymbolSpace::Type => {
                        type_symbols.entry(name).or_insert(*symbol_id);
                    }
                    SymbolSpace::Value => {
                        value_symbols.entry(name).or_insert(*symbol_id);
                    }
                    SymbolSpace::TypeValue => {
                        type_value_symbols.entry(name).or_insert(*symbol_id);
                    }
                    SymbolSpace::Label => {}
                }
            }

            // select preferred symbols for this scope
            let mut scope_preferred: IndexMap<StringId, LocalSymbolId> = IndexMap::new();
            for name in scope_fallback.keys() {
                let preferred = space_order.spaces().iter().find_map(|space| match space {
                    SymbolSpace::Type => type_symbols
                        .get(name)
                        .copied()
                        .or(type_value_symbols.get(name).copied()),
                    SymbolSpace::Value => value_symbols
                        .get(name)
                        .copied()
                        .or(type_value_symbols.get(name).copied()),
                    SymbolSpace::TypeValue => type_value_symbols.get(name).copied(),
                    SymbolSpace::Label => None,
                });
                if let Some(symbol_id) = preferred {
                    scope_preferred.insert(*name, symbol_id);
                }
            }

            // merge this scope into the module maps
            for (name, symbol_id) in scope_fallback {
                fallback_symbols.entry(name).or_insert(symbol_id);
            }
            for (name, symbol_id) in scope_preferred {
                resolved_symbols.entry(name).or_insert(symbol_id);
            }

            // move to the parent scope when available
            if let Some((parent_scope_id, parent_mark)) = scope.parent {
                scope_id = parent_scope_id;
                scope_mark = parent_mark;
            }
            // stop when there is no parent scope
            else {
                break;
            }
        }

        // fall back to nearest symbols when no preferred match exists
        for (name, symbol_id) in fallback_symbols {
            resolved_symbols.entry(name).or_insert(symbol_id);
        }

        resolved_symbols
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
    use destack_builtin::{LIBS, LanguageSymbol, STD_LIB};
    use destack_dir::{WellKnownSymbol, WellKnownSymbolKey};

    use crate::{TestProgram, assert_string};

    /// Test that language item modules can be looked up correctly.
    #[test]
    fn test_resolve_language_symbol_symbol() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.resolve_builtins();
        test.compile();

        let language_symbol_id = test.compiler.language_symbol(LanguageSymbol::Add);
        let language_symbol_module = test.program.modules.get(language_symbol_id.module_id);
        let language_symbol_module = language_symbol_module.read();
        let profile = test.default_profile_id(language_symbol_id.module_id);
        let language_symbol_symbols = language_symbol_module.dir(profile).symbols.read();
        let language_symbol_symbol =
            language_symbol_symbols.get_symbol(language_symbol_id.into_local());
        assert_string!(test.program, language_symbol_symbol.name().unwrap(), "Add");
    }

    /// Resolve declared lib symbols for all builtin libs.
    #[test]
    fn test_resolve_builtin_lib_symbols() {
        for lib in LIBS {
            let lib_names = [lib.name];
            let test = TestProgram::memory_sequential_with_prelude_and_libs()
                .with_profile_libs(&lib_names);
            test.resolve_builtins();
            test.resolve_libs();
            test.compile();

            let profile = test.default_profile_id_for_root();
            for &symbol in lib.declared_symbols {
                let name_id = test.program.strings.intern(symbol);
                assert!(
                    test.compiler
                        .get_declared_lib_symbol(profile, name_id)
                        .is_some(),
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
