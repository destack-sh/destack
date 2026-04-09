use std::collections::HashSet;
use std::path::Path;

use dashmap::DashMap;
use destack_artifact::{
    EmitFormat, Platform, ProfileKey, Runtime, TargetArch, TargetEnv, TargetVendor,
};
use destack_builtin::{
    BuiltinLibraryKind, BuiltinLibrarySource, BuiltinPlatform, BuiltinRuntime, INTRINSIC_SOURCES,
    LanguageSymbol, PRELUDE_BUILTIN_SOURCE, builtin_libraries, builtin_library,
    resolve_profile_builtin_library_name,
};
use destack_source::{FileContent, ModuleId, PackageId};
use indexmap::IndexMap;
use parking_lot::Mutex;

use crate::ModuleSource;
use crate::repository::{FileOrigin, Ref, Repository, RepositoryError, Revision, SourceMap};

/// Well-known package ID for builtins.
pub const BUILTIN_PACKAGE_ID: PackageId = PackageId(1);

/// Well-known package name for builtins.
pub const BUILTIN_PACKAGE_NAME: &str = "@destack/builtin";

/// Key for builtin library and symbol data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BuiltinLibraryKey {
    /// Emit format for builtin selection.
    emit: EmitFormat,
    /// Runtime for builtin selection.
    runtime: Runtime,
    /// Platform for builtin selection.
    platform: Platform,
    /// Target architecture for builtin selection.
    target_arch: Option<TargetArch>,
    /// Target vendor for builtin selection.
    target_vendor: Option<TargetVendor>,
    /// Target environment for builtin selection.
    target_env: Option<TargetEnv>,
    /// Normalized library set.
    lib: Vec<String>,
}

impl BuiltinLibraryKey {
    /// Build a builtin key from a full profile key.
    fn from_profile_key(profile_key: &ProfileKey) -> Self {
        Self {
            emit: profile_key.emit,
            runtime: profile_key.runtime,
            platform: profile_key.platform,
            target_arch: profile_key.target_arch.clone(),
            target_vendor: profile_key.target_vendor.clone(),
            target_env: profile_key.target_env.clone(),
            lib: profile_key.lib.clone(),
        }
    }
}

/// Input-side builtin library selection for one profile key.
#[derive(Debug, Clone, Default)]
pub struct BuiltinLibrarySelection {
    /// Builtin libraries in dependency order.
    pub ordered_libraries: Vec<String>,
    /// Module batches in library dependency order.
    pub modules_to_resolve: Vec<Vec<ModuleId>>,
    /// All selected library modules in load order without duplicates.
    pub library_modules: Vec<ModuleId>,
    /// Ambient library modules in load order without duplicates.
    pub ambient_modules: Vec<ModuleId>,
}

/// Immutable builtin module catalog.
#[derive(Debug)]
struct BuiltinCatalog {
    /// Intrinsic modules by path.
    intrinsic_module_by_path: IndexMap<String, ModuleId>,
    /// Intrinsic module for each language item.
    intrinsic_module_by_item: IndexMap<LanguageSymbol, ModuleId>,
    /// The prelude module id.
    prelude_module_id: ModuleId,
    /// The builtin source for each module.
    module_source_by_id: DashMap<ModuleId, ModuleSource>,
    /// The builtin logical path for each module.
    module_path_by_id: DashMap<ModuleId, String>,
}

impl BuiltinCatalog {
    /// Create one empty builtin catalog.
    fn empty() -> Self {
        Self {
            intrinsic_module_by_path: IndexMap::new(),
            intrinsic_module_by_item: IndexMap::new(),
            prelude_module_id: ModuleId::EPHEMERAL,
            module_source_by_id: DashMap::new(),
            module_path_by_id: DashMap::new(),
        }
    }
}

/// Mutable builtin library cache state.
#[derive(Debug)]
struct BuiltinLibraryCache {
    /// Normalized builtin library names for one profile key and input name.
    resolved_name_by_input: DashMap<(BuiltinLibraryKey, String), String>,
    /// Library modules cache.
    module_ids_by_name: DashMap<(BuiltinLibraryKey, String), Vec<ModuleId>>,
    /// Selected builtin library modules for one profile key.
    selection_by_key: DashMap<BuiltinLibraryKey, BuiltinLibrarySelection>,
    /// Input-side lock for builtin library registration.
    load_lock: Mutex<()>,
    /// Library name for each registered library module.
    library_name_by_module: DashMap<ModuleId, &'static str>,
}

impl BuiltinLibraryCache {
    /// Create one empty builtin library cache.
    fn empty() -> Self {
        Self {
            resolved_name_by_input: DashMap::new(),
            module_ids_by_name: DashMap::new(),
            selection_by_key: DashMap::new(),
            load_lock: Mutex::new(()),
            library_name_by_module: DashMap::new(),
        }
    }
}

/// Language and library builtins.
#[derive(Debug)]
pub struct Builtins {
    /// The immutable builtin module catalog.
    catalog: BuiltinCatalog,
    /// The mutable builtin library cache.
    library_cache: BuiltinLibraryCache,
}

impl Builtins {
    /// Create one empty builtin store.
    pub fn empty() -> Self {
        Self {
            catalog: BuiltinCatalog::empty(),
            library_cache: BuiltinLibraryCache::empty(),
        }
    }

    /// Register intrinsic builtin metadata.
    pub fn load_intrinsics(&mut self) {
        let mut intrinsic_modules = IndexMap::with_capacity(INTRINSIC_SOURCES.len());
        for source in INTRINSIC_SOURCES {
            let module_path = source.module_path();
            let module_id =
                ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(&module_path));
            self.catalog.module_source_by_id.insert(
                module_id,
                ModuleSource::Builtin(BuiltinLibraryKind::Intrinsic),
            );
            self.catalog
                .module_path_by_id
                .insert(module_id, module_path.clone());
            intrinsic_modules.insert(module_path, module_id);
        }

        for library in builtin_libraries() {
            for source in library.sources {
                let module_path = source.module_path();
                let module_id =
                    ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(&module_path));
                self.catalog
                    .module_source_by_id
                    .insert(module_id, ModuleSource::Builtin(library.kind));
                self.catalog
                    .module_path_by_id
                    .insert(module_id, module_path);
            }
        }

        let mut language_symbol_modules = IndexMap::with_capacity(LanguageSymbol::all().count());
        for item in LanguageSymbol::all() {
            let module_path = format!("{}.ds", item.module());
            let module_id = intrinsic_modules.get(&module_path).unwrap_or_else(|| {
                panic!(
                    "language item {item:?} references module '{module_path}' which is not in INTRINSIC_SOURCES"
                )
            });
            language_symbol_modules.insert(item, *module_id);
        }

        let prelude_path = PRELUDE_BUILTIN_SOURCE.module_path();
        let prelude_module_id = *intrinsic_modules.get(&prelude_path).unwrap_or_else(|| {
            panic!("prelude module '{prelude_path}' is not in INTRINSIC_SOURCES")
        });

        self.catalog.intrinsic_module_by_path = intrinsic_modules;
        self.catalog.intrinsic_module_by_item = language_symbol_modules;
        self.catalog.prelude_module_id = prelude_module_id;
    }

    /// Return the builtin prelude module id.
    pub fn prelude_module_id(&self) -> ModuleId {
        self.catalog.prelude_module_id
    }

    /// Get the ModuleId for a language item's defining module.
    pub fn module_for_item(&self, item: LanguageSymbol) -> ModuleId {
        *self
            .catalog
            .intrinsic_module_by_item
            .get(&item)
            .unwrap_or_else(|| panic!("language item {item:?} not registered"))
    }

    /// Return the builtin source kind for one module.
    pub fn module_source(&self, module_id: ModuleId) -> Option<ModuleSource> {
        self.catalog
            .module_source_by_id
            .get(&module_id)
            .map(|source| *source.value())
    }

    /// Return the builtin logical path for one module.
    pub fn module_path(&self, module_id: ModuleId) -> Option<String> {
        self.catalog
            .module_path_by_id
            .get(&module_id)
            .map(|path| path.value().clone())
    }

    /// Return the builtin module source kind for one builtin module path.
    pub fn module_source_for_path(&self, module_path: &str) -> Option<ModuleSource> {
        let module_id = ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(module_path));
        self.module_source(module_id)
    }

    /// Return the cached builtin library selection for one profile key.
    pub fn cached_library_selection(
        &self,
        profile_key: &ProfileKey,
    ) -> Option<BuiltinLibrarySelection> {
        let library_key = BuiltinLibraryKey::from_profile_key(profile_key);
        self.library_cache
            .selection_by_key
            .get(&library_key)
            .map(|selection| selection.clone())
    }

    /// Return whether one builtin library has sources for one profile key.
    pub fn has_library_for_profile(&self, name: &str, profile_key: &ProfileKey) -> bool {
        let Some(library) = builtin_library(name) else {
            return false;
        };

        let runtime = BuiltinRuntime::from(profile_key.runtime);
        let emit = profile_key.emit.builtin_output_format();
        let platform = BuiltinPlatform::from(profile_key.platform);

        library
            .sources
            .iter()
            .any(|source| source.matches_target(runtime, emit, platform))
    }

    /// Cache the builtin library selection for one profile key.
    pub fn cache_library_selection(
        &self,
        profile_key: &ProfileKey,
        selection: BuiltinLibrarySelection,
    ) {
        let library_key = BuiltinLibraryKey::from_profile_key(profile_key);
        self.library_cache
            .selection_by_key
            .insert(library_key, selection);
    }

    /// Load a library module set.
    pub fn load_library(&self, name: &str, profile_key: &ProfileKey) -> Option<Vec<ModuleId>> {
        let library_key = BuiltinLibraryKey::from_profile_key(profile_key);
        let name = self.resolved_library_name(name, profile_key, &library_key);
        let _guard = self.library_cache.load_lock.lock();

        let mut loading = HashSet::new();
        self.load_library_inner(&name, profile_key, &library_key, &mut loading)
    }

    /// Install builtin source into one repository ref.
    pub fn install_in_repository(
        &self,
        repository: &Repository,
        reference: &Ref,
    ) -> Result<Revision, RepositoryError> {
        let mut seeded_source = SourceMap::new();

        // intrinsic builtin sources
        for builtin_source in INTRINSIC_SOURCES {
            repository.insert_seeded_source(
                &mut seeded_source,
                FileOrigin::builtin(builtin_source.module_path()),
                FileContent::Text {
                    content: builtin_source.content.to_string(),
                },
            );
        }

        // library builtin sources
        for library in builtin_libraries() {
            for builtin_source in library.sources {
                repository.insert_seeded_source(
                    &mut seeded_source,
                    FileOrigin::builtin(builtin_source.module_path()),
                    FileContent::Text {
                        content: builtin_source.content.to_string(),
                    },
                );
            }
        }

        repository.apply_seeded_source(reference, seeded_source)
    }

    /// Get the library name for a module, if any.
    pub fn library_name_for_module(&self, module_id: ModuleId) -> Option<&'static str> {
        self.library_cache
            .library_name_by_module
            .get(&module_id)
            .map(|name| *name)
    }

    /// Return the path inside one builtin library for one module.
    pub fn library_relative_path_for_module(&self, module_id: ModuleId) -> Option<String> {
        let library_name = self.library_name_for_module(module_id)?;
        let module_source = self.module_source(module_id)?;
        let module_path = self.module_path(module_id)?;

        let prefix = match module_source {
            ModuleSource::Builtin(BuiltinLibraryKind::Language) => {
                format!("language/{library_name}/")
            }
            ModuleSource::Builtin(BuiltinLibraryKind::Library) => {
                format!("library/{library_name}/")
            }
            ModuleSource::Builtin(BuiltinLibraryKind::Intrinsic) | ModuleSource::User => {
                return None;
            }
        };

        module_path
            .strip_prefix(&prefix)
            .map(|relative_path| relative_path.to_string())
    }

    /// Get builtin modules that define intrinsic bindings.
    pub fn intrinsic_module_ids(&self) -> Vec<ModuleId> {
        let mut module_ids = Vec::new();
        let mut seen = HashSet::new();

        for (path, module_id) in self.catalog.intrinsic_module_by_path.iter() {
            if !path.starts_with("primitive/") {
                continue;
            }

            if seen.insert(*module_id) {
                module_ids.push(*module_id);
            }
        }

        module_ids
    }

    /// Get builtin modules referenced by language item definitions.
    pub fn language_symbol_module_ids(&self) -> Vec<ModuleId> {
        let mut module_ids = Vec::new();
        let mut seen = HashSet::new();

        for module_id in self.catalog.intrinsic_module_by_item.values().copied() {
            if seen.insert(module_id) {
                module_ids.push(module_id);
            }
        }

        module_ids
    }

    /// Get all registered intrinsic builtin modules.
    pub fn registered_intrinsic_module_ids(&self) -> Vec<ModuleId> {
        let mut module_ids = Vec::new();
        let mut seen = HashSet::new();

        for module_id in self.catalog.intrinsic_module_by_path.values().copied() {
            if seen.insert(module_id) {
                module_ids.push(module_id);
            }
        }

        module_ids
    }

    fn load_library_inner(
        &self,
        name: &str,
        profile_key: &ProfileKey,
        library_key: &BuiltinLibraryKey,
        loading: &mut HashSet<String>,
    ) -> Option<Vec<ModuleId>> {
        if let Some(cached) = self.cached_library_modules(name, library_key) {
            return Some(cached);
        }

        if loading.contains(name) {
            return Some(Vec::new());
        }

        let library = builtin_library(name)?;
        let runtime = BuiltinRuntime::from(profile_key.runtime);
        let emit = profile_key.emit.builtin_output_format();
        let platform = BuiltinPlatform::from(profile_key.platform);
        let filtered_sources = library
            .sources
            .iter()
            .copied()
            .filter(|source| source.matches_target(runtime, emit, platform))
            .collect::<Vec<_>>();
        if filtered_sources.is_empty() {
            return None;
        }

        loading.insert(name.to_string());

        let mut dependencies = Vec::new();
        let mut seen_dependencies = HashSet::new();

        for &dependency in library.dependencies {
            let dependency = self.resolved_library_name(dependency, profile_key, library_key);
            if seen_dependencies.insert(dependency.clone()) {
                dependencies.push(dependency);
            }
        }

        for &reference in library.reference_libs {
            let reference = self.resolved_library_name(reference, profile_key, library_key);
            if seen_dependencies.insert(reference.clone()) {
                dependencies.push(reference);
            }
        }

        for dependency in dependencies {
            if dependency == library.name {
                continue;
            }

            if self
                .load_library_inner(&dependency, profile_key, library_key, loading)
                .is_none()
            {
                loading.remove(name);
                return None;
            }
        }

        let mut module_ids = Vec::with_capacity(filtered_sources.len());
        for source in filtered_sources {
            let module_id = self.module_id_for_library_source(&source);
            module_ids.push(module_id);
        }

        for module_id in &module_ids {
            self.library_cache
                .library_name_by_module
                .insert(*module_id, library.name);
        }

        self.library_cache
            .module_ids_by_name
            .insert((library_key.clone(), name.to_string()), module_ids.clone());

        loading.remove(name);

        Some(module_ids)
    }

    fn resolved_library_name(
        &self,
        name: &str,
        profile_key: &ProfileKey,
        library_key: &BuiltinLibraryKey,
    ) -> String {
        let cache_key = (library_key.clone(), name.to_string());

        if let Some(cached) = self.library_cache.resolved_name_by_input.get(&cache_key) {
            return cached.clone();
        }

        let resolved = resolve_profile_builtin_library_name(name, &profile_key.lib)
            .unwrap_or_else(|| name.to_string());

        self.library_cache
            .resolved_name_by_input
            .insert(cache_key, resolved.clone());

        resolved
    }

    fn cached_library_modules(
        &self,
        name: &str,
        library_key: &BuiltinLibraryKey,
    ) -> Option<Vec<ModuleId>> {
        let cached = self
            .library_cache
            .module_ids_by_name
            .get(&(library_key.clone(), name.to_string()))?;

        if let Some(library) = builtin_library(name) {
            for module_id in cached.iter() {
                self.library_cache
                    .library_name_by_module
                    .insert(*module_id, library.name);
            }
        }

        Some(cached.clone())
    }

    /// Return the deterministic module id for one builtin source.
    pub fn module_id_for_library_source(&self, source: &BuiltinLibrarySource) -> ModuleId {
        let module_path = source.module_path();
        ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(&module_path))
    }
}
