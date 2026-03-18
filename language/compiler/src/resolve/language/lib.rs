use std::collections::{HashMap, HashSet};

use destack_builtin::builtin_lib;
use destack_core::StringId;
use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder};
use destack_workspace::{
    BuiltinLibSelection, Builtins, LibEnvironment, LibSymbolKey, ProfileId, ProfileKey,
    SymbolGroup, WellKnownSymbols,
};
use indexmap::IndexMap;

use crate::resolve::module::globals::{GlobalSymbolGroupKey, GlobalSymbolTable};
use crate::timing::tags;
use crate::{BuildRequirementCollector, Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the lib environment for one profile.
    pub fn resolve_lib_environment(&self, profile_id: ProfileId) -> ResolveResult<LibEnvironment> {
        if self.lib_environment(profile_id).is_some() {
            return Ok(self
                .lib_environment(profile_id)
                .unwrap_or_else(|| unreachable!())
                .as_ref()
                .clone());
        }

        if !self.options.load_libs {
            return Ok(LibEnvironment::default());
        }

        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(LibEnvironment::default());
        };
        let _timing = self.timing_scope(tags::RESOLVE_LIBS);

        self.require_language_environment(profile_id)
            .map_err(ResolveError::from)?;

        // collect libs to resolve
        let profile_key = self.program.profile(profile_id).key.clone();
        let BuiltinLibSelection {
            ordered_libs,
            modules_to_resolve,
            lib_modules,
            ambient_modules,
        } = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_DEPENDENCIES);
            self.builtin_lib_selection(builtins, &profile_key)?
        };

        // resolve selected lib surfaces in order
        let mut collector = BuildRequirementCollector::new();
        for module_ids in &modules_to_resolve {
            for &module_id in module_ids {
                if let Err(error) = self.require_dir_resolved(module_id, profile_id)
                    && let Some(error) = collector.try_collect::<(), _>(Err(error))
                {
                    let requirement = error.into_requirement();
                    return Err(ResolveError::UnsatisfiedRequirement { requirement });
                }
            }
        }
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        // require declared lib surfaces before building the environment
        let mut collector = BuildRequirementCollector::new();
        for &module_id in &lib_modules {
            if let Err(error) = self.require_dir_declared(module_id, profile_id)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                let requirement = error.into_requirement();
                return Err(ResolveError::UnsatisfiedRequirement { requirement });
            }
        }
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        // build global symbol cache for lib modules
        let global_cache = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_GLOBAL_CACHE);
            self.build_global_symbol_table_freestanding(&lib_modules, profile_id)?
        };

        // find declared symbol names from lib definitions
        let declared_names = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_DECLARED_NAMES);
            self.collect_declared_lib_symbol_names(&ordered_libs)?
        };
        let declared_symbols = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_DECLARED_SYMBOLS);
            self.find_declared_lib_symbols(profile_id, &lib_modules, &global_cache, &declared_names)
        };
        let symbols = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_AMBIENT_SYMBOLS);
            self.collect_lib_symbols(profile_id, &lib_modules, &global_cache)
        };
        let symbol_sources = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_AMBIENT_SOURCES);
            self.collect_lib_symbol_sources(profile_id, &lib_modules, &global_cache)
        };
        let well_known_symbols = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_WELL_KNOWN);
            WellKnownSymbols::build(&self.program.strings, &declared_symbols)
        };

        Ok(LibEnvironment {
            modules: lib_modules,
            ambient_modules,
            declared_symbols,
            symbols,
            symbol_sources,
            well_known_symbols,
        })
    }

    /// Collect selected builtin lib modules from profile input state.
    pub(crate) fn selected_lib_modules_from_input(
        &self,
        profile_id: ProfileId,
    ) -> ResolveResult<Vec<destack_source::ModuleId>> {
        if let Some(environment) = self.lib_environment(profile_id) {
            return Ok(environment.supporting_modules());
        }

        if !self.options.load_libs {
            return Ok(Vec::new());
        }

        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(Vec::new());
        };

        let profile_key = self.program.profile(profile_id).key.clone();
        let selection = self.builtin_lib_selection(builtins, &profile_key)?;

        Ok(selection.lib_modules)
    }

    /// Collect ambient builtin lib modules from profile input state.
    pub(crate) fn ambient_lib_modules_from_input(
        &self,
        profile_id: ProfileId,
    ) -> ResolveResult<Vec<destack_source::ModuleId>> {
        if let Some(environment) = self.lib_environment(profile_id) {
            return Ok(environment.ambient_modules.clone());
        }

        if !self.options.load_libs {
            return Ok(Vec::new());
        }

        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(Vec::new());
        };

        let profile_key = self.program.profile(profile_id).key.clone();
        let selection = self.builtin_lib_selection(builtins, &profile_key)?;

        Ok(selection.ambient_modules)
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

    /// Find declared lib symbols from the global cache and direct lib surfaces.
    /// Returns a map with SymbolGroup entries containing both type and value symbols.
    fn find_declared_lib_symbols(
        &self,
        profile_id: ProfileId,
        lib_modules: &[destack_source::ModuleId],
        global_cache: &GlobalSymbolTable,
        declared_names: &HashSet<StringId>,
    ) -> IndexMap<StringId, SymbolGroup> {
        let mut declared_symbols = IndexMap::new();

        for &name_id in declared_names {
            let key = StaticKey::Name(name_id);

            // try global cache first for both spaces
            let type_key = GlobalSymbolGroupKey {
                key,
                space: SymbolSpace::Type,
            };
            let value_key = GlobalSymbolGroupKey {
                key,
                space: SymbolSpace::Value,
            };
            let type_value_key = GlobalSymbolGroupKey {
                key,
                space: SymbolSpace::TypeValue,
            };
            let type_symbol = global_cache
                .symbols_by_space
                .get(&type_key)
                .or_else(|| global_cache.symbols_by_space.get(&type_value_key))
                .copied();
            let value_symbol = global_cache
                .symbols_by_space
                .get(&value_key)
                .or_else(|| global_cache.symbols_by_space.get(&type_value_key))
                .copied();
            if type_symbol.is_some() || value_symbol.is_some() {
                declared_symbols.insert(
                    name_id,
                    SymbolGroup {
                        ty: type_symbol,
                        value: value_symbol,
                    },
                );
                continue;
            }

            // read declared exports directly when the cache did not provide them
            let mut group = SymbolGroup::default();
            for &module_id in lib_modules {
                // load the module
                let module = self.program.modules.get(module_id);
                let dir = self
                    .require_artifact_dir_resolved(module_id, profile_id)
                    .map_err(ResolveError::from)
                    .unwrap_or_else(|_| unreachable!());
                let symbols = &dir.symbols;
                let exports = &dir.exported_symbols;
                let tree = &dir.tree;

                // resolve declared namespace scope entries first
                if group.ty.is_none() || group.value.is_none() {
                    let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
                    for (scope_key, symbol_id) in symbols.active_named_symbols(namespace_scope) {
                        if scope_key != key {
                            continue;
                        }

                        let symbol = symbols.get_symbol(symbol_id);
                        let target_id = symbol_id.into_global(module_id);
                        match symbol.space {
                            SymbolSpace::Type => {
                                if group.ty.is_none() {
                                    group.ty = Some(target_id);
                                }
                            }
                            SymbolSpace::Value => {
                                if group.value.is_none() {
                                    group.value = Some(target_id);
                                }
                            }
                            SymbolSpace::TypeValue => {
                                if group.ty.is_none() {
                                    group.ty = Some(target_id);
                                }
                                if group.value.is_none() {
                                    group.value = Some(target_id);
                                }
                            }
                            SymbolSpace::Label => {}
                        }
                    }
                }

                // resolve declared global augmentation entries next
                if group.ty.is_none() || group.value.is_none() {
                    let global_scope = symbols.get_scope_by_id(dir.global_augmentation_scope);
                    for (scope_key, symbol_id) in symbols.active_named_symbols(global_scope) {
                        if scope_key != key {
                            continue;
                        }

                        let symbol = symbols.get_symbol(symbol_id);
                        let target_id = symbol_id.into_global(module_id);
                        match symbol.space {
                            SymbolSpace::Type => {
                                if group.ty.is_none() {
                                    group.ty = Some(target_id);
                                }
                            }
                            SymbolSpace::Value => {
                                if group.value.is_none() {
                                    group.value = Some(target_id);
                                }
                            }
                            SymbolSpace::TypeValue => {
                                if group.ty.is_none() {
                                    group.ty = Some(target_id);
                                }
                                if group.value.is_none() {
                                    group.value = Some(target_id);
                                }
                            }
                            SymbolSpace::Label => {}
                        }
                    }
                }

                // resolve type-space symbol if not yet found
                if group.ty.is_none()
                    && let Some(symbol_id) = self.resolve_exported_symbol(
                        &module,
                        profile_id,
                        &exports,
                        &tree,
                        SymbolSpaceOrder::TypeOnly,
                        key,
                    )
                {
                    group.ty = Some(symbol_id);
                }

                // resolve value-space symbol if not yet found
                if group.value.is_none()
                    && let Some(symbol_id) = self.resolve_exported_symbol(
                        &module,
                        profile_id,
                        &exports,
                        &tree,
                        SymbolSpaceOrder::ValueOnly,
                        key,
                    )
                {
                    group.value = Some(symbol_id);
                }

                // stop once both are found
                if group.ty.is_some() && group.value.is_some() {
                    break;
                }
            }

            if !group.is_empty() {
                declared_symbols.insert(name_id, group);
            }
        }

        declared_symbols
    }

    /// Collect all exported symbols from selected lib modules.
    fn collect_lib_symbols(
        &self,
        profile_id: ProfileId,
        lib_modules: &[destack_source::ModuleId],
        global_cache: &GlobalSymbolTable,
    ) -> IndexMap<StringId, SymbolGroup> {
        let mut symbols = IndexMap::new();

        // first, collect from global cache (by space)
        for (group_key, &symbol_id) in &global_cache.symbols_by_space {
            let StaticKey::Name(name_id) = group_key.key else {
                continue;
            };
            let group = symbols.entry(name_id).or_insert(SymbolGroup::default());
            match group_key.space {
                SymbolSpace::Type => {
                    if group.ty.is_none() {
                        group.ty = Some(symbol_id);
                    }
                }
                SymbolSpace::Value => {
                    if group.value.is_none() {
                        group.value = Some(symbol_id);
                    }
                }
                SymbolSpace::TypeValue => {
                    if group.ty.is_none() {
                        group.ty = Some(symbol_id);
                    }
                    if group.value.is_none() {
                        group.value = Some(symbol_id);
                    }
                }
                SymbolSpace::Label => {}
            }
        }

        // then, supplement from module scopes and exports
        for &module_id in lib_modules {
            let dir = self
                .require_artifact_dir_resolved(module_id, profile_id)
                .map_err(ResolveError::from)
                .unwrap_or_else(|_| unreachable!());
            let symbols_table = &dir.symbols;
            let exports = &dir.exported_symbols;

            // collect namespace scope declarations directly
            let namespace_scope = symbols_table.get_scope_by_id(dir.namespace_scope);
            for (key, symbol_id) in symbols_table.active_named_symbols(namespace_scope) {
                let StaticKey::Name(name_id) = key else {
                    continue;
                };

                let group = symbols.entry(name_id).or_insert(SymbolGroup::default());
                let symbol = symbols_table.get_symbol(symbol_id);
                let target_id = symbol_id.into_global(module_id);

                match symbol.space {
                    SymbolSpace::Type => {
                        if group.ty.is_none() {
                            group.ty = Some(target_id);
                        }
                    }
                    SymbolSpace::Value => {
                        if group.value.is_none() {
                            group.value = Some(target_id);
                        }
                    }
                    SymbolSpace::TypeValue => {
                        if group.ty.is_none() {
                            group.ty = Some(target_id);
                        }
                        if group.value.is_none() {
                            group.value = Some(target_id);
                        }
                    }
                    SymbolSpace::Label => {}
                }
            }

            // collect global augmentation declarations directly
            let global_scope = symbols_table.get_scope_by_id(dir.global_augmentation_scope);
            for (key, symbol_id) in symbols_table.active_named_symbols(global_scope) {
                let StaticKey::Name(name_id) = key else {
                    continue;
                };

                let group = symbols.entry(name_id).or_insert(SymbolGroup::default());
                let symbol = symbols_table.get_symbol(symbol_id);
                let target_id = symbol_id.into_global(module_id);

                match symbol.space {
                    SymbolSpace::Type => {
                        if group.ty.is_none() {
                            group.ty = Some(target_id);
                        }
                    }
                    SymbolSpace::Value => {
                        if group.value.is_none() {
                            group.value = Some(target_id);
                        }
                    }
                    SymbolSpace::TypeValue => {
                        if group.ty.is_none() {
                            group.ty = Some(target_id);
                        }
                        if group.value.is_none() {
                            group.value = Some(target_id);
                        }
                    }
                    SymbolSpace::Label => {}
                }
            }

            // exported_symbols is IndexMap<(SymbolSpace, StaticKey), Export>
            for ((space, key), export) in exports.iter() {
                let StaticKey::Name(name_id) = *key else {
                    continue;
                };

                // resolve the export target
                let Some(target_id) = export.target.resolved() else {
                    continue;
                };

                // add to the appropriate slot in the group
                let group = symbols.entry(name_id).or_insert(SymbolGroup::default());
                match space {
                    SymbolSpace::Type => {
                        if group.ty.is_none() {
                            group.ty = Some(target_id);
                        }
                    }
                    SymbolSpace::Value => {
                        if group.value.is_none() {
                            group.value = Some(target_id);
                        }
                    }
                    SymbolSpace::TypeValue => {
                        if group.ty.is_none() {
                            group.ty = Some(target_id);
                        }
                        if group.value.is_none() {
                            group.value = Some(target_id);
                        }
                    }
                    SymbolSpace::Label => {}
                }
            }
        }

        symbols
    }

    /// Collect selected lib symbol sources grouped by key and space.
    fn collect_lib_symbol_sources(
        &self,
        profile_id: ProfileId,
        lib_modules: &[destack_source::ModuleId],
        global_cache: &GlobalSymbolTable,
    ) -> IndexMap<LibSymbolKey, Vec<GlobalSymbolId>> {
        let mut sources = IndexMap::new();
        let lib_set: HashSet<_> = lib_modules.iter().copied().collect();

        for (group_key, group_sources) in &global_cache.sources_by_space {
            let mut filtered = Vec::new();
            for symbol in group_sources {
                if lib_set.contains(&symbol.module_id) {
                    filtered.push(*symbol);
                }
            }

            if filtered.is_empty() {
                continue;
            }

            sources.insert(
                LibSymbolKey {
                    key: group_key.key,
                    space: group_key.space,
                },
                filtered,
            );
        }

        // supplement merge sources from module scopes and exports
        for &module_id in lib_modules {
            let dir = self
                .require_artifact_dir_resolved(module_id, profile_id)
                .map_err(ResolveError::from)
                .unwrap_or_else(|_| unreachable!());
            let symbols_table = &dir.symbols;
            let exports = &dir.exported_symbols;

            // collect namespace scope declarations directly
            let namespace_scope = symbols_table.get_scope_by_id(dir.namespace_scope);
            for (key, symbol_id) in symbols_table.active_named_symbols(namespace_scope) {
                let symbol = symbols_table.get_symbol(symbol_id);
                let group_key = LibSymbolKey {
                    key,
                    space: symbol.space,
                };
                let group_sources = sources.entry(group_key).or_default();
                let target_id = symbol_id.into_global(module_id);

                if !group_sources.contains(&target_id) {
                    group_sources.push(target_id);
                }
            }

            // collect global augmentation declarations directly
            let global_scope = symbols_table.get_scope_by_id(dir.global_augmentation_scope);
            for (key, symbol_id) in symbols_table.active_named_symbols(global_scope) {
                let symbol = symbols_table.get_symbol(symbol_id);
                let group_key = LibSymbolKey {
                    key,
                    space: symbol.space,
                };
                let group_sources = sources.entry(group_key).or_default();
                let target_id = symbol_id.into_global(module_id);

                if !group_sources.contains(&target_id) {
                    group_sources.push(target_id);
                }
            }

            for ((space, key), export) in exports.iter() {
                let Some(symbol_id) = export.target.resolved() else {
                    continue;
                };

                let group_key = LibSymbolKey {
                    key: *key,
                    space: *space,
                };
                let group_sources = sources.entry(group_key).or_default();

                if !group_sources.contains(&symbol_id) {
                    group_sources.push(symbol_id);
                }
            }
        }

        sources
    }

    /// Resolve the builtin lib selection for one profile key.
    fn builtin_lib_selection(
        &self,
        builtins: &Builtins,
        profile_key: &ProfileKey,
    ) -> ResolveResult<BuiltinLibSelection> {
        if let Some(selection) = builtins.lib_selection(profile_key) {
            return Ok(selection);
        }

        // dependency order
        let ordered_libs = self.ordered_libs_for_profile_key(profile_key)?;
        if self.check_builtin_lib_version_conflicts(&ordered_libs)? {
            let selection = BuiltinLibSelection::default();
            builtins.set_lib_selection(profile_key, selection.clone());
            return Ok(selection);
        }

        // load libs in order
        let mut ambient_modules = Vec::new();
        let mut ambient_seen = HashSet::new();
        let mut all_modules = Vec::new();
        let mut all_seen = HashSet::new();
        let mut modules_to_resolve = Vec::new();

        for lib_name in &ordered_libs {
            // load the lib definition
            let lib = builtin_lib(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLib {
                name: lib_name.clone(),
            })?;

            // load the lib modules
            let module_ids = builtins.load_lib(
                lib_name,
                self.program.files.clone(),
                self.program.modules.clone(),
                profile_key,
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

        let selection = BuiltinLibSelection {
            ordered_libs,
            modules_to_resolve,
            lib_modules: all_modules,
            ambient_modules,
        };

        builtins.set_lib_selection(profile_key, selection.clone());

        Ok(selection)
    }

    /// Build the ordered builtin lib set for one profile key.
    fn ordered_libs_for_profile_key(&self, profile_key: &ProfileKey) -> ResolveResult<Vec<String>> {
        let mut libs = profile_key.lib.clone();

        // always include std lib
        if !libs.contains(&"std".to_string()) {
            libs.push("std".to_string());
        }

        // always include globals
        if !libs.contains(&"globals".to_string()) {
            libs.push("globals".to_string());
        }

        // dependency order
        let version_overrides = collect_lib_version_overrides(&libs)?;
        let mut ordered_libs = Vec::new();
        let mut seen_libs = HashSet::new();
        for lib_name in &libs {
            self.collect_lib_dependencies(
                lib_name,
                &mut ordered_libs,
                &mut seen_libs,
                &version_overrides,
            )?;
        }

        Ok(ordered_libs)
    }

    /// Load builtin lib modules in dependency order for benchmark runs.
    #[cfg(feature = "bench")]
    pub(crate) fn load_lib_modules_for_bench(
        &self,
        libs: &[&str],
    ) -> ResolveResult<Vec<destack_source::ModuleId>> {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(Vec::new());
        };

        let mut lib_names: Vec<String> = libs.iter().map(|lib| (*lib).to_string()).collect();
        if !lib_names.iter().any(|name| name == "std") {
            lib_names.push("std".to_string());
        }
        if !lib_names.iter().any(|name| name == "globals") {
            lib_names.push("globals".to_string());
        }

        let default_profile_id = self
            .program
            .default_profile_id_for_module(self.program.root_module_id);
        let mut profile_key = self.program.profile(default_profile_id).key.clone();
        profile_key.lib = lib_names;

        let selection = self.builtin_lib_selection(builtins, &profile_key)?;
        Ok(selection.lib_modules)
    }

    /// Collect the dependencies of a lib and add them to the ordered list.
    fn collect_lib_dependencies(
        &self,
        name: &str,
        ordered: &mut Vec<String>,
        seen: &mut HashSet<String>,
        version_overrides: &HashMap<String, String>,
    ) -> ResolveResult<()> {
        // skip already collected libs
        if !seen.insert(name.to_string()) {
            return Ok(());
        }

        // get the lib
        let lib = builtin_lib(name).ok_or_else(|| ResolveError::MissingBuiltinLib {
            name: name.to_string(),
        })?;

        // collect dependencies
        for &dependency in lib.dependencies {
            let dependency = resolve_lib_dependency_name(dependency, version_overrides);
            self.collect_lib_dependencies(&dependency, ordered, seen, version_overrides)?;
        }

        // collect reference lib dependencies
        for dependency in lib.reference_libs() {
            let dependency = resolve_lib_dependency_name(dependency, version_overrides);
            self.collect_lib_dependencies(&dependency, ordered, seen, version_overrides)?;
        }

        // record the lib after dependencies
        ordered.push(name.to_string());
        Ok(())
    }

    /// Reject multiple builtin lib versions in the same lib set.
    fn check_builtin_lib_version_conflicts(&self, ordered_libs: &[String]) -> ResolveResult<bool> {
        // group libs by base and version
        let mut grouped = HashMap::<String, HashMap<String, HashSet<String>>>::new();

        for name in ordered_libs {
            let Some((base, version)) = builtin_lib_version_info(name) else {
                continue;
            };
            grouped
                .entry(base)
                .or_default()
                .entry(version)
                .or_default()
                .insert(name.clone());
        }

        // collect bases with multiple versions
        let mut conflicts: Vec<(String, Vec<String>)> = grouped
            .into_iter()
            .filter_map(|(base, versions)| {
                if versions.len() <= 1 {
                    return None;
                }
                let mut names: Vec<String> = versions
                    .into_iter()
                    .flat_map(|(_, names)| names.into_iter())
                    .collect();
                names.sort();
                names.dedup();
                Some((base, names))
            })
            .collect();

        // report the first conflict if any
        conflicts.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some((base, libs)) = conflicts.first() {
            self.error(ResolveError::ConflictingBuiltinLibVersions {
                base: base.clone(),
                libs: libs.join(", "),
            });
            return Ok(true);
        }

        Ok(false)
    }
}

/// Return the base name and version for a builtin lib, if versioned.
fn builtin_lib_version_info(name: &str) -> Option<(String, String)> {
    // prefer explicit versioned names
    if let Some((base, version)) = split_versioned_lib_name(name) {
        return Some((base.to_string(), version.to_string()));
    }

    // fall back to the lib source path
    let lib = builtin_lib(name)?;
    let source = lib.sources.first()?;
    split_versioned_path(source.path)
}

/// Split a name like "node.v24" into base and version.
fn split_versioned_lib_name(name: &str) -> Option<(&str, &str)> {
    // find the version separator
    let index = name.find(".v")?;
    let version = &name[index + 2..];
    if version.is_empty() {
        return None;
    }

    // require a digit to avoid alias segments
    let is_versioned = version.chars().next().is_some_and(|ch| ch.is_ascii_digit());
    if !is_versioned {
        return None;
    }

    Some((&name[..index], version))
}

/// Split a source path like "node/v24" into base and version.
fn split_versioned_path(path: &str) -> Option<(String, String)> {
    // read base and version path segments
    let mut segments = path.split('/');
    let base = segments.next()?;
    let version = segments.next_back()?;
    let version = version.strip_prefix('v')?;

    // require a digit to treat as versioned
    let is_versioned = version.chars().next().is_some_and(|ch| ch.is_ascii_digit());
    if is_versioned {
        return Some((base.to_string(), version.to_string()));
    }

    None
}

/// Collect explicit lib version overrides from the requested lib list.
fn collect_lib_version_overrides(libs: &[String]) -> ResolveResult<HashMap<String, String>> {
    // prepare traversal state
    let mut overrides = HashMap::new();
    let mut seen = HashSet::new();

    // walk all requested libs
    for lib in libs {
        collect_lib_version_overrides_for_lib(lib, &mut overrides, &mut seen)?;
    }

    Ok(overrides)
}

/// Collect explicit versioned libs in the dependency graph.
fn collect_lib_version_overrides_for_lib(
    name: &str,
    overrides: &mut HashMap<String, String>,
    seen: &mut HashSet<String>,
) -> ResolveResult<()> {
    // skip already visited libs
    if !seen.insert(name.to_string()) {
        return Ok(());
    }

    // record explicit versioned libs
    if let Some((base, _version)) = split_versioned_lib_name(name) {
        overrides
            .entry(base.to_string())
            .or_insert_with(|| name.to_string());
    }

    // load the lib for dependencies
    let lib = builtin_lib(name).ok_or_else(|| ResolveError::MissingBuiltinLib {
        name: name.to_string(),
    })?;

    // walk dependencies
    for &dependency in lib.dependencies {
        collect_lib_version_overrides_for_lib(dependency, overrides, seen)?;
    }

    Ok(())
}

/// Resolve a dependency name to a versioned lib when an override exists.
fn resolve_lib_dependency_name(name: &str, version_overrides: &HashMap<String, String>) -> String {
    // keep explicit versioned dependencies
    if split_versioned_lib_name(name).is_some() {
        return name.to_string();
    }

    // apply any explicit version override
    if let Some(override_name) = version_overrides.get(name) {
        return override_name.clone();
    }

    // fall back to the original name
    name.to_string()
}
