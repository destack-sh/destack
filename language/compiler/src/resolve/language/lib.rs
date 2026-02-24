use std::collections::{HashMap, HashSet};

use destack_base::StringId;
use destack_builtin::builtin_lib;
use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder};
use destack_workspace::{
    AmbientLibSymbolKey, Builtins, GlobalSymbolGroupKey, GlobalSymbolTable, ProfileId, ProfileKey,
    SymbolGroup, WellKnownSymbols,
};
use indexmap::IndexMap;

use crate::resolve::dependency::cache::ResolveDependencyItemCache;
use crate::timing::tags;
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

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a profile's libraries.
    pub fn resolve_libs(&self, profile_id: ProfileId) -> ResolveResult<()> {
        if !self.options.load_libs {
            return Ok(());
        }
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(());
        };
        let _timing = self.timing_scope(tags::RESOLVE_LIBS);

        self.require_resolve_builtins(profile_id)?;

        // collect libs to resolve
        let profile = self.program.profile(profile_id);
        let profile_key = profile.key.clone();

        // skip when this profile key already has resolved lib caches
        if builtins.ambient_libs(&profile_key).is_some()
            && builtins
                .declared_lib_symbols_by_profile
                .contains_key(&profile_key)
            && builtins
                .ambient_lib_symbols_by_profile
                .contains_key(&profile_key)
            && builtins
                .ambient_lib_symbol_sources_by_profile
                .contains_key(&profile_key)
            && builtins.well_known_symbols(&profile_key).is_some()
        {
            return Ok(());
        }

        let mut libs = profile_key.lib.clone();

        // always include std lib (?)
        if !libs.contains(&"std".to_string()) {
            libs.push("std".to_string());
        }

        // always include globals
        if !libs.contains(&"globals".to_string()) {
            libs.push("globals".to_string());
        }

        // build lib load order with dependencies first
        let version_overrides = collect_lib_version_overrides(&libs)?;
        let ordered_libs = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_DEPENDENCIES);
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
            ordered_libs
        };

        // reject conflicting builtin lib versions before loading modules
        {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_CONFLICTS);
            self.check_builtin_lib_version_conflicts(&ordered_libs)?;
        }

        // load libs in order
        let loaded_modules = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_LOAD_MODULES);
            self.load_lib_modules_in_order(builtins, &ordered_libs, &profile_key)?
        };
        let LoadedLibModules {
            modules_to_resolve,
            lib_modules,
            ambient_modules,
        } = loaded_modules;
        builtins.set_ambient_libs(&profile_key, ambient_modules.clone());

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
        {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_DEPENDENCY_ITEMS);
            let mut dependency_cache = ResolveDependencyItemCache::default();
            for &module_id in &lib_modules {
                if let Err(error) = self.resolve_dependency_items_with_cache(
                    module_id,
                    profile_id,
                    &mut dependency_cache,
                ) && let Some(error) = collector.try_collect::<(), _>(Err(error))
                {
                    return Err(error);
                }
            }
            if let Some(dependency) = collector.try_into_yield_all() {
                return Err(ResolveError::Yield { dependency });
            }
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

        // collect all ambient lib symbols
        let ambient_symbols = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_AMBIENT_SYMBOLS);
            self.collect_ambient_lib_symbols(profile_id, &ambient_modules, &global_cache)
        };
        let ambient_symbol_sources = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_AMBIENT_SOURCES);
            self.collect_ambient_lib_symbol_sources(&ambient_modules, &global_cache)
        };

        // cache declared lib symbols
        builtins.set_declared_lib_symbols(&profile_key, declared_symbols.clone());

        // cache ambient lib symbols
        builtins.set_ambient_lib_symbols(&profile_key, ambient_symbols);
        builtins.set_ambient_lib_symbol_sources(&profile_key, ambient_symbol_sources);

        // cache well-known symbols
        let well_known_symbols = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_WELL_KNOWN);
            WellKnownSymbols::build(&self.program.strings, &declared_symbols)
        };
        builtins.set_well_known_symbols(&profile_key, well_known_symbols);

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

            // fall back to exports, looking for both type and value
            let mut group = SymbolGroup::default();
            for &module_id in lib_modules {
                // load the module
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let dir = module.dir(profile_id);
                let exports = dir.exported_symbols.read();
                let tree = dir.tree.read();

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

    /// Collect all exported symbols from ambient lib modules.
    fn collect_ambient_lib_symbols(
        &self,
        profile_id: ProfileId,
        ambient_modules: &[destack_source::ModuleId],
        global_cache: &GlobalSymbolTable,
    ) -> IndexMap<StringId, SymbolGroup> {
        let mut ambient_symbols = IndexMap::new();

        // first, collect from global cache (by space)
        for (group_key, &symbol_id) in &global_cache.symbols_by_space {
            let StaticKey::Name(name_id) = group_key.key else {
                continue;
            };
            let group = ambient_symbols
                .entry(name_id)
                .or_insert(SymbolGroup::default());
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

        // then, supplement from module exports
        for &module_id in ambient_modules {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir(profile_id);
            let exports = dir.exported_symbols.read();

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
                let group = ambient_symbols
                    .entry(name_id)
                    .or_insert(SymbolGroup::default());
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

        ambient_symbols
    }

    /// Collect ambient lib symbol sources grouped by key and space.
    fn collect_ambient_lib_symbol_sources(
        &self,
        ambient_modules: &[destack_source::ModuleId],
        global_cache: &GlobalSymbolTable,
    ) -> IndexMap<AmbientLibSymbolKey, Vec<GlobalSymbolId>> {
        let mut sources = IndexMap::new();
        let ambient_set: HashSet<_> = ambient_modules.iter().copied().collect();

        for (group_key, group_sources) in &global_cache.sources_by_space {
            let mut filtered = Vec::new();
            for symbol in group_sources {
                if ambient_set.contains(&symbol.module_id) {
                    filtered.push(*symbol);
                }
            }

            if filtered.is_empty() {
                continue;
            }

            sources.insert(
                AmbientLibSymbolKey {
                    key: group_key.key,
                    space: group_key.space,
                },
                filtered,
            );
        }

        sources
    }

    /// Load lib modules in dependency order and collect module lists.
    fn load_lib_modules_in_order(
        &self,
        builtins: &Builtins,
        ordered_libs: &[String],
        profile_key: &ProfileKey,
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

        Ok(LoadedLibModules {
            modules_to_resolve,
            lib_modules: all_modules,
            ambient_modules,
        })
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

        let version_overrides = collect_lib_version_overrides(&lib_names)?;
        let mut ordered_libs = Vec::new();
        let mut seen_libs = HashSet::new();
        for lib_name in &lib_names {
            self.collect_lib_dependencies(
                lib_name,
                &mut ordered_libs,
                &mut seen_libs,
                &version_overrides,
            )?;
        }

        let default_profile_id = self
            .program
            .default_profile_id_for_module(self.program.root_module_id);
        let mut profile_key = self.program.profile(default_profile_id).key.clone();
        profile_key.lib = lib_names;

        self.check_builtin_lib_version_conflicts(&ordered_libs)?;
        let loaded = self.load_lib_modules_in_order(builtins, &ordered_libs, &profile_key)?;
        Ok(loaded.lib_modules)
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
    fn check_builtin_lib_version_conflicts(&self, ordered_libs: &[String]) -> ResolveResult<()> {
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
            return Err(ResolveError::ConflictingBuiltinLibVersions {
                base: base.clone(),
                libs: libs.join(", "),
            });
        }

        Ok(())
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
