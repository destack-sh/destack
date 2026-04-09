use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, CanonicalStaticKey, LibraryEnvironment, LibrarySymbolKey, ProfileKey,
};
use destack_builtin::builtin_library;
use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;
use destack_workspace::{BuiltinLibrarySelection, Builtins, ProfileId, Revision};
use indexmap::IndexMap;

use crate::resolve::module::globals::GlobalSymbolTable;
use crate::timing::tags;
use crate::{Compiler, RequirementCollector, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the library environment for one profile.
    pub(crate) fn resolve_library_environment(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> ResolveResult<LibraryEnvironment> {
        if let Some(environment) = self.repository.library_environment(revision, profile_id) {
            return Ok(environment.as_ref().clone());
        }

        let artifact_key = ArtifactKey::library_environment(profile_id);
        match self.load_library_environment_image(revision, profile_id) {
            Ok(Some(environment)) => return Ok(environment),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(?error, ?artifact_key, "compiler.cache.image.load_failed");
            }
        }

        if !self.options.load_libraries {
            return Ok(LibraryEnvironment::default());
        }

        let builtins = self.repository.builtins.as_ref();
        let _timing = self.timing_scope(tags::RESOLVE_LIBS);

        self.require_language_environment(revision, profile_id)
            .map_err(ResolveError::from)?;

        // collect libraries to resolve
        let profile_key = self.profile(profile_id).key.clone();
        let BuiltinLibrarySelection {
            ordered_libraries,
            modules_to_resolve,
            library_modules,
            ambient_modules,
        } = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_DEPENDENCIES);
            self.builtin_library_selection(builtins, &profile_key)?
        };

        // resolve selected library surfaces in order
        let mut collector = RequirementCollector::new();
        for module_ids in &modules_to_resolve {
            for &module_id in module_ids {
                if let Err(error) = self.require_dir_resolved(revision, module_id, profile_id)
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

        // build the global symbol cache for library modules
        let global_cache = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_GLOBAL_CACHE);
            self.build_global_symbol_table_freestanding(revision, &library_modules, profile_id)?
        };

        // collect selected symbol source facts once
        let symbol_sources = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_AMBIENT_SOURCES);
            self.collect_library_symbol_sources(
                revision,
                profile_id,
                &library_modules,
                &global_cache,
            )
        };
        let declared_symbol_sources = {
            let _timing = self.timing_scope(tags::RESOLVE_LIBS_DECLARED_SYMBOLS);
            self.collect_declared_library_symbol_sources(&ordered_libraries, &symbol_sources)?
        };

        Ok(LibraryEnvironment {
            modules: library_modules,
            ambient_modules,
            declared_library_symbol_sources: declared_symbol_sources,
            library_symbol_sources: symbol_sources,
        })
    }

    /// Collect selected builtin library modules from profile input state.
    pub(crate) fn selected_library_modules_from_input(
        &self,
        profile_id: ProfileId,
    ) -> ResolveResult<Arc<[ModuleId]>> {
        if !self.options.load_libraries {
            return Ok(Arc::<[ModuleId]>::from([]));
        }

        // compiler cache
        if let Some(modules) = self.selected_library_modules_by_profile.get(&profile_id) {
            let modules: Arc<[ModuleId]> = Arc::clone(modules.value());

            return Ok(modules);
        }

        let builtins = self.repository.builtins.as_ref();

        let profile_key = self.profile(profile_id).key.clone();
        let selection = self.builtin_library_selection(builtins, &profile_key)?;
        let modules = Arc::<[ModuleId]>::from(selection.library_modules);

        self.selected_library_modules_by_profile
            .insert(profile_id, Arc::clone(&modules));

        Ok(modules)
    }

    /// Collect ambient builtin library modules from profile input state.
    pub(crate) fn ambient_library_modules_from_input(
        &self,
        profile_id: ProfileId,
    ) -> ResolveResult<Arc<[ModuleId]>> {
        if !self.options.load_libraries {
            return Ok(Arc::<[ModuleId]>::from([]));
        }

        // compiler cache
        if let Some(modules) = self.ambient_library_modules_by_profile.get(&profile_id) {
            let modules: Arc<[ModuleId]> = Arc::clone(modules.value());

            return Ok(modules);
        }

        let builtins = self.repository.builtins.as_ref();

        let profile_key = self.profile(profile_id).key.clone();
        let selection = self.builtin_library_selection(builtins, &profile_key)?;
        let modules = Arc::<[ModuleId]>::from(selection.ambient_modules);

        self.ambient_library_modules_by_profile
            .insert(profile_id, Arc::clone(&modules));

        Ok(modules)
    }

    /// Collect the declared symbol names from builtin libraries.
    fn collect_declared_library_symbol_names(
        &self,
        ordered_libraries: &[String],
    ) -> ResolveResult<HashSet<String>> {
        // collect declared symbol names from all libraries
        let mut declared_names = HashSet::new();
        for lib_name in ordered_libraries {
            let library =
                builtin_library(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLibrary {
                    name: lib_name.clone(),
                })?;
            for &name in library.declared_symbols {
                declared_names.insert(name.to_string());
            }
        }

        Ok(declared_names)
    }

    /// Filter declared library symbol sources from the selected library source facts.
    fn collect_declared_library_symbol_sources(
        &self,
        ordered_libraries: &[String],
        symbol_sources: &IndexMap<LibrarySymbolKey, Vec<GlobalSymbolId>>,
    ) -> ResolveResult<IndexMap<LibrarySymbolKey, Vec<GlobalSymbolId>>> {
        let declared_names = self.collect_declared_library_symbol_names(ordered_libraries)?;
        let mut declared_symbol_sources = IndexMap::new();

        for (key, sources) in symbol_sources {
            let CanonicalStaticKey::Name(name) = &key.key else {
                continue;
            };

            if !declared_names.contains(name.as_str()) {
                continue;
            }

            declared_symbol_sources.insert(key.clone(), sources.clone());
        }

        Ok(declared_symbol_sources)
    }

    /// Collect selected library symbol sources grouped by key and space.
    fn collect_library_symbol_sources(
        &self,
        revision: Revision,
        profile_id: ProfileId,
        library_modules: &[destack_source::ModuleId],
        global_cache: &GlobalSymbolTable,
    ) -> IndexMap<LibrarySymbolKey, Vec<GlobalSymbolId>> {
        let mut sources = IndexMap::new();
        let library_set: HashSet<_> = library_modules.iter().copied().collect();

        for (group_key, group_sources) in &global_cache.sources_by_space {
            let mut filtered = Vec::new();
            for symbol in group_sources {
                if library_set.contains(&symbol.module_id) {
                    filtered.push(*symbol);
                }
            }

            if filtered.is_empty() {
                continue;
            }

            sources.insert(
                LibrarySymbolKey {
                    key: CanonicalStaticKey::from_static_key(
                        group_key.key,
                        &self.repository.strings,
                    ),
                    space: group_key.space,
                },
                filtered,
            );
        }

        // supplement merge sources from module scopes and exports
        for &module_id in library_modules {
            let dir = self
                .require_artifact_dir_resolved(revision, module_id, profile_id)
                .map_err(ResolveError::from)
                .unwrap_or_else(|_| unreachable!());
            let symbols_table = &dir.symbols;
            let exports = &dir.exported_symbols;

            // collect namespace scope declarations directly
            let namespace_scope = symbols_table.get_scope_by_id(dir.namespace_scope);
            for (key, symbol_id) in symbols_table.active_named_symbols(namespace_scope) {
                let symbol = symbols_table.get_symbol(symbol_id);
                let group_key = LibrarySymbolKey {
                    key: CanonicalStaticKey::from_static_key(key, &self.repository.strings),
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
                let group_key = LibrarySymbolKey {
                    key: CanonicalStaticKey::from_static_key(key, &self.repository.strings),
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

                let group_key = LibrarySymbolKey {
                    key: CanonicalStaticKey::from_static_key(*key, &self.repository.strings),
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

    /// Resolve the builtin library selection for one profile key.
    pub(crate) fn builtin_library_selection(
        &self,
        builtins: &Builtins,
        profile_key: &ProfileKey,
    ) -> ResolveResult<BuiltinLibrarySelection> {
        if let Some(selection) = builtins.cached_library_selection(profile_key) {
            return Ok(selection);
        }

        // dependency order
        let ordered_libraries = self.ordered_libraries_for_profile_key(profile_key)?;
        if self.check_builtin_library_version_conflicts(&ordered_libraries)? {
            let selection = BuiltinLibrarySelection::default();
            builtins.cache_library_selection(profile_key, selection.clone());
            return Ok(selection);
        }

        // load libraries in order
        let mut ambient_modules = Vec::new();
        let mut ambient_seen = HashSet::new();
        let mut all_modules = Vec::new();
        let mut all_seen = HashSet::new();
        let mut modules_to_resolve = Vec::new();

        for lib_name in &ordered_libraries {
            // load the library definition
            let library =
                builtin_library(lib_name).ok_or_else(|| ResolveError::MissingBuiltinLibrary {
                    name: lib_name.clone(),
                })?;

            // load the library modules
            let module_ids = self.repository.load_builtin_library(lib_name, profile_key);
            let module_ids = module_ids.ok_or_else(|| ResolveError::MissingBuiltinLibrary {
                name: lib_name.clone(),
            })?;
            modules_to_resolve.push(module_ids.clone());

            // collect into the appropriate lists
            for &module_id in &module_ids {
                if all_seen.insert(module_id) {
                    all_modules.push(module_id);
                }
                if library.is_ambient && ambient_seen.insert(module_id) {
                    ambient_modules.push(module_id);
                }
            }
        }

        let selection = BuiltinLibrarySelection {
            ordered_libraries,
            modules_to_resolve,
            library_modules: all_modules,
            ambient_modules,
        };

        builtins.cache_library_selection(profile_key, selection.clone());

        Ok(selection)
    }

    /// Build the ordered builtin library set for one profile key.
    fn ordered_libraries_for_profile_key(
        &self,
        profile_key: &ProfileKey,
    ) -> ResolveResult<Vec<String>> {
        let mut libs = profile_key.lib.clone();

        // always include globals
        if !libs.contains(&"globals".to_string()) {
            libs.push("globals".to_string());
        }

        // dependency order
        let version_overrides = collect_library_version_overrides(&libs)?;
        let mut ordered_libraries = Vec::new();
        let mut seen_libs = HashSet::new();
        for lib_name in &libs {
            self.collect_library_dependencies(
                lib_name,
                &mut ordered_libraries,
                &mut seen_libs,
                &version_overrides,
            )?;
        }

        Ok(ordered_libraries)
    }

    /// Collect the dependencies of a library and add them to the ordered list.
    fn collect_library_dependencies(
        &self,
        name: &str,
        ordered: &mut Vec<String>,
        seen: &mut HashSet<String>,
        version_overrides: &HashMap<String, String>,
    ) -> ResolveResult<()> {
        // skip already collected libraries
        if !seen.insert(name.to_string()) {
            return Ok(());
        }

        // get the library
        let library = builtin_library(name).ok_or_else(|| ResolveError::MissingBuiltinLibrary {
            name: name.to_string(),
        })?;

        // collect dependencies
        for &dependency in library.dependencies {
            let dependency = resolve_library_dependency_name(dependency, version_overrides);
            self.collect_library_dependencies(&dependency, ordered, seen, version_overrides)?;
        }

        // collect reference library dependencies
        for &dependency in library.reference_libs {
            let dependency = resolve_library_dependency_name(dependency, version_overrides);
            self.collect_library_dependencies(&dependency, ordered, seen, version_overrides)?;
        }

        // record the library after dependencies
        ordered.push(name.to_string());
        Ok(())
    }

    /// Reject multiple builtin library versions in the same library set.
    fn check_builtin_library_version_conflicts(
        &self,
        ordered_libraries: &[String],
    ) -> ResolveResult<bool> {
        // group libraries by base and version
        let mut grouped = HashMap::<String, HashMap<String, HashSet<String>>>::new();

        for name in ordered_libraries {
            let Some((base, version)) = builtin_library_version_info(name) else {
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
            self.error(ResolveError::ConflictingBuiltinLibraryVersions {
                base: base.clone(),
                libs: libs.join(", "),
            });
            return Ok(true);
        }

        Ok(false)
    }
}

/// Return the base name and version for a builtin library, if versioned.
fn builtin_library_version_info(name: &str) -> Option<(String, String)> {
    // prefer explicit versioned names
    if let Some((base, version)) = split_versioned_library_name(name) {
        return Some((base.to_string(), version.to_string()));
    }

    // fall back to the library source path
    let library = builtin_library(name)?;
    let source = library.sources.first()?;
    split_versioned_path(source.path)
}

/// Split a name like "node.v24" into base and version.
fn split_versioned_library_name(name: &str) -> Option<(&str, &str)> {
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

/// Collect explicit library version overrides from the requested library list.
fn collect_library_version_overrides(libs: &[String]) -> ResolveResult<HashMap<String, String>> {
    // prepare traversal state
    let mut overrides = HashMap::new();
    let mut seen = HashSet::new();

    // walk all requested libraries
    for lib in libs {
        collect_library_version_overrides_for_library(lib, &mut overrides, &mut seen)?;
    }

    Ok(overrides)
}

/// Collect explicit versioned libraries in the dependency graph.
fn collect_library_version_overrides_for_library(
    name: &str,
    overrides: &mut HashMap<String, String>,
    seen: &mut HashSet<String>,
) -> ResolveResult<()> {
    // skip already visited libraries
    if !seen.insert(name.to_string()) {
        return Ok(());
    }

    // record explicit versioned libraries
    if let Some((base, _version)) = split_versioned_library_name(name) {
        overrides
            .entry(base.to_string())
            .or_insert_with(|| name.to_string());
    }

    // load the library for dependencies
    let library = builtin_library(name).ok_or_else(|| ResolveError::MissingBuiltinLibrary {
        name: name.to_string(),
    })?;

    // walk dependencies
    for &dependency in library.dependencies {
        collect_library_version_overrides_for_library(dependency, overrides, seen)?;
    }

    Ok(())
}

/// Resolve a dependency name to a versioned library when an override exists.
fn resolve_library_dependency_name(
    name: &str,
    version_overrides: &HashMap<String, String>,
) -> String {
    // keep explicit versioned dependencies
    if split_versioned_library_name(name).is_some() {
        return name.to_string();
    }

    // apply any explicit version override
    if let Some(override_name) = version_overrides.get(name) {
        return override_name.clone();
    }

    // fall back to the original name
    name.to_string()
}
