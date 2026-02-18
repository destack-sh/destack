use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use destack_base::{StringId, StringPool};
use destack_builtin::{
    BuiltinLibKind, BuiltinLibSource, BuiltinOutputFormat, BuiltinPlatform, BuiltinRuntime,
    CORE_SOURCES, LanguageSymbol, PRELUDE_SOURCE, builtin_lib,
};
use destack_dir::{
    GlobalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder, WellKnownSymbol, WellKnownSymbolKey,
};
use destack_source::{
    File, FileRegistry, FileType, LanguageType, ModuleId, PackageId, PackageVersion, Uri,
};
use indexmap::IndexMap;

use crate::{
    Loader, Module, ModuleFormat, ModuleRegistry, ModuleSource, Package, PackageKind,
    PackageRegistry, ProfileId, ProfileKey, SourceType,
};

/// A symbol group containing type and value space entries.
/// Used to track both spaces for dual-space symbols like interfaces with constructors.
#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolGroup {
    /// The type-space symbol, if any.
    pub ty: Option<GlobalSymbolId>,
    /// The value-space symbol, if any.
    pub value: Option<GlobalSymbolId>,
}

/// A symbol key for ambient lib sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AmbientLibSymbolKey {
    /// The symbol key.
    pub key: StaticKey,
    /// The symbol space.
    pub space: SymbolSpace,
}

impl SymbolGroup {
    /// Create a new SymbolGroup from a type symbol.
    pub fn from_type(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: Some(symbol),
            value: None,
        }
    }

    /// Create a new SymbolGroup from a value symbol.
    pub fn from_value(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: None,
            value: Some(symbol),
        }
    }

    /// Create a new SymbolGroup from a type-value symbol (stored in both).
    pub fn from_type_value(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: Some(symbol),
            value: Some(symbol),
        }
    }

    /// Check if the group is empty.
    pub fn is_empty(&self) -> bool {
        self.ty.is_none() && self.value.is_none()
    }

    /// Get the symbol for the given space order preference.
    pub fn symbol_for_space_order(&self, order: SymbolSpaceOrder) -> Option<GlobalSymbolId> {
        match order {
            SymbolSpaceOrder::None => None,
            SymbolSpaceOrder::TypeOnly => self.ty,
            SymbolSpaceOrder::ValueOnly => self.value,
            SymbolSpaceOrder::TypeThenValue => self.ty.or(self.value),
            SymbolSpaceOrder::ValueThenType => self.value.or(self.ty),
        }
    }

    /// Merge another group into this one, overwriting None values.
    pub fn merge(&mut self, other: SymbolGroup) {
        if other.ty.is_some() {
            self.ty = other.ty;
        }
        if other.value.is_some() {
            self.value = other.value;
        }
    }
}

/// Resolve a file type for a builtin source name.
fn file_type_for_builtin_name(name: &str) -> FileType {
    FileType::from_path(Path::new(name)).unwrap_or(FileType::Destack)
}

/// Well-known package ID for builtins.
pub const BUILTIN_PACKAGE_ID: PackageId = PackageId(1);

/// Well-known package name for builtins.
pub const BUILTIN_PACKAGE_NAME: &str = "@destack/builtin";

/// Well-known symbol key metadata.
#[derive(Debug, Clone, Copy)]
pub struct WellKnownKey {
    /// The base symbol for the key.
    pub symbol: GlobalSymbolId,
    /// The member name on the base symbol.
    pub member: StringId,
    /// The full global key name (like "Symbol.iterator").
    pub global_name: StringId,
}

/// Resolved compiler-known symbols for a profile.
#[derive(Debug, Clone)]
pub struct WellKnownSymbols {
    /// Top-level builtin symbols by well-known id.
    pub symbols: IndexMap<WellKnownSymbol, SymbolGroup>,
    /// Well-known symbol keys by id.
    pub keys: IndexMap<WellKnownSymbolKey, WellKnownKey>,
}

impl WellKnownSymbols {
    /// Build the well-known symbol map from resolved lib symbols.
    pub fn build(strings: &StringPool, lib_symbols: &IndexMap<StringId, SymbolGroup>) -> Self {
        let mut symbols = IndexMap::new();
        let mut keys = IndexMap::new();

        for item in WellKnownSymbol::all() {
            let name_id = strings.intern(item.export_name());
            let Some(group) = lib_symbols.get(&name_id).copied() else {
                continue;
            };
            if group.is_empty() {
                continue;
            }
            symbols.insert(item, group);
        }

        // for well-known keys, use the value symbol as the base (for member access)
        for item in WellKnownSymbolKey::all() {
            let base_symbol = item.base_symbol();
            let Some(group) = symbols.get(&base_symbol) else {
                continue;
            };
            let Some(base_symbol_id) = group.value.or(group.ty) else {
                continue;
            };
            let member_id = strings.intern(item.member_name());
            let global_name_id = strings.intern(item.global_symbol_name());
            keys.insert(
                item,
                WellKnownKey {
                    symbol: base_symbol_id,
                    member: member_id,
                    global_name: global_name_id,
                },
            );
        }

        Self { symbols, keys }
    }

    /// Get the symbol group for a well-known symbol.
    pub fn get_group(&self, item: WellKnownSymbol) -> Option<SymbolGroup> {
        self.symbols.get(&item).copied()
    }

    /// Get the value-space well-known symbol by id (for backwards compatibility).
    pub fn get_symbol(&self, item: WellKnownSymbol) -> Option<GlobalSymbolId> {
        self.symbols
            .get(&item)
            .and_then(|group| group.value.or(group.ty))
    }

    /// Get the type-space well-known symbol by id.
    pub fn get_type_symbol(&self, item: WellKnownSymbol) -> Option<GlobalSymbolId> {
        self.symbols
            .get(&item)
            .and_then(|group| group.ty.or(group.value))
    }

    /// Get the value-space well-known symbol by id.
    pub fn get_value_symbol(&self, item: WellKnownSymbol) -> Option<GlobalSymbolId> {
        self.symbols.get(&item).and_then(|group| group.value)
    }

    /// Get a well-known key by id.
    pub fn get_key(&self, item: WellKnownSymbolKey) -> Option<&WellKnownKey> {
        self.keys.get(&item)
    }

    /// Resolve a well-known key id for a base symbol and member name.
    pub fn symbol_key_for_member(
        &self,
        symbol: GlobalSymbolId,
        member: StringId,
    ) -> Option<WellKnownSymbolKey> {
        self.keys.iter().find_map(|(key_id, key)| {
            if key.symbol == symbol && key.member == member {
                return Some(*key_id);
            }
            None
        })
    }
}

/// Resolved compiler-known intrinsic bindings for a profile.
#[derive(Debug, Clone)]
pub struct WellKnownIntrinsics {
    /// Intrinsic names keyed by symbol id.
    pub names_by_symbol: IndexMap<GlobalSymbolId, StringId>,
    /// Intrinsic symbols keyed by name.
    pub symbols_by_name: IndexMap<StringId, GlobalSymbolId>,
}

impl WellKnownIntrinsics {
    /// Create an empty well-known intrinsic map.
    pub fn new() -> Self {
        Self {
            names_by_symbol: IndexMap::new(),
            symbols_by_name: IndexMap::new(),
        }
    }

    /// Resolve an intrinsic name for a symbol.
    pub fn name_for_symbol(&self, symbol: GlobalSymbolId) -> Option<StringId> {
        self.names_by_symbol.get(&symbol).copied()
    }

    /// Resolve a symbol for an intrinsic name.
    pub fn symbol_for_name(&self, name: StringId) -> Option<GlobalSymbolId> {
        self.symbols_by_name.get(&name).copied()
    }
}

impl Default for WellKnownIntrinsics {
    fn default() -> Self {
        Self::new()
    }
}

/// Language and library builtins.
#[derive(Debug)]
pub struct Builtins {
    /// The builtin package ID.
    pub package_id: PackageId,
    /// Core modules by path (e.g., "operator/arithmetic.ds" -> ModuleId).
    pub core_module_by_path: IndexMap<String, ModuleId>,
    /// Core module for each language item (LanguageSymbol -> ModuleId).
    pub core_module_by_item: IndexMap<LanguageSymbol, ModuleId>,
    /// The prelude module ID (re-exports items available without imports).
    pub prelude_module_id: ModuleId,

    /// Lib modules cache ((profile, "dom") -> modules).
    pub lib_module_by_name: DashMap<(ProfileKey, String), Vec<ModuleId>>,
    /// Lib load markers by name.
    /// #Cleanup: can we do better than lib load markers in Builtins?
    /// (It's a little annoying fishy, but we have to protect against concurrent re-entrant loads.)
    pub lib_loading_by_name: DashMap<(ProfileKey, String), ()>,
    /// Lib name for each registered lib module.
    pub lib_name_by_module: DashMap<ModuleId, &'static str>,
    /// Ambient lib modules per profile key.
    pub ambient_libs_by_profile: DashMap<ProfileKey, Vec<ModuleId>>,

    /// Declared lib symbols per profile key.
    pub declared_lib_symbols_by_profile: DashMap<ProfileKey, IndexMap<StringId, SymbolGroup>>,
    /// Ambient lib symbols per profile key (all exported symbols from ambient libs).
    pub ambient_lib_symbols_by_profile: DashMap<ProfileKey, IndexMap<StringId, SymbolGroup>>,
    /// Ambient lib symbol sources per profile key (all occurrences by key and space).
    pub ambient_lib_symbol_sources_by_profile:
        DashMap<ProfileKey, IndexMap<AmbientLibSymbolKey, Vec<GlobalSymbolId>>>,
    /// Well-known symbols per profile key.
    pub well_known_by_profile: DashMap<ProfileKey, WellKnownSymbols>,
    /// Well-known intrinsic bindings per profile key.
    pub well_known_intrinsics_by_profile: DashMap<ProfileKey, WellKnownIntrinsics>,

    /// Resolved language items cache (ProfileId, LanguageSymbol -> GlobalSymbolId).
    pub items: DashMap<(ProfileId, LanguageSymbol), GlobalSymbolId>,
}

impl Builtins {
    /// Create builtins by registering core modules from embedded sources.
    pub fn embedded(
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        packages: Arc<PackageRegistry>,
    ) -> Self {
        // create builtin package
        let package = Package {
            id: BUILTIN_PACKAGE_ID,
            package_version: PackageVersion::INITIAL,
            kind: PackageKind::Builtin,
            uri: Uri::from_string("builtin://"),
            path: None,
            name: Some(BUILTIN_PACKAGE_NAME.to_string()),
            version: None,
            manifest: None,
            dsconfig: None,
            tsconfig: None,
            targets: Default::default(),
        };
        packages.insert(package);

        // register core modules from embedded sources
        let mut core_modules = IndexMap::with_capacity(CORE_SOURCES.len());
        for source in CORE_SOURCES {
            let uri = Uri::from_string(source.virtual_path());

            // select file/language types for the builtin source
            let file_type = file_type_for_builtin_name(source.name);
            let language_type = LanguageType::from(file_type);

            // create file
            let file_id = files.next_id();
            let file = File::from_text(
                file_id,
                source.name.to_string(),
                uri.clone(),
                None,
                file_type,
                source.content.to_string(),
            );
            let file_version = file.version;
            files.insert(file);

            // create module id from path
            let module_path = if source.path.is_empty() {
                source.name.to_string()
            } else {
                format!("{}/{}", source.path, source.name)
            };
            let module_id = ModuleId::from_relative_path(
                BUILTIN_PACKAGE_ID,
                std::path::Path::new(&module_path),
            );

            // create blank module (will be parsed/bound later)
            let loader = Loader::from_file_type(file_type);
            let module = Module::blank(
                module_id,
                file_id,
                file_version,
                uri,
                None,
                BUILTIN_PACKAGE_ID,
                None,
                SourceType::Module,
                ModuleFormat::Esm,
                language_type,
                loader,
                ModuleSource::Builtin(BuiltinLibKind::Core),
            );
            modules.insert(module);
            core_modules.insert(module_path, module_id);
        }

        // build language item -> module mapping, validating all items have modules
        let mut language_symbol_modules = IndexMap::with_capacity(LanguageSymbol::all().count());
        for item in LanguageSymbol::all() {
            let module_path = format!("{}.ds", item.module());
            let module_id = core_modules.get(&module_path).unwrap_or_else(|| {
                panic!(
                    "language item {item:?} references module '{module_path}' which is not in CORE_SOURCES"
                )
            });
            language_symbol_modules.insert(item, *module_id);
        }

        // look up prelude module ID (must be registered)
        let prelude_path = if PRELUDE_SOURCE.path.is_empty() {
            PRELUDE_SOURCE.name.to_string()
        } else {
            format!("{}/{}", PRELUDE_SOURCE.path, PRELUDE_SOURCE.name)
        };
        let prelude_module_id = *core_modules
            .get(&prelude_path)
            .unwrap_or_else(|| panic!("prelude module '{prelude_path}' is not in CORE_SOURCES"));

        Self {
            package_id: BUILTIN_PACKAGE_ID,
            core_module_by_path: core_modules,
            core_module_by_item: language_symbol_modules,
            prelude_module_id,
            lib_module_by_name: DashMap::new(),
            lib_loading_by_name: DashMap::new(),
            lib_name_by_module: DashMap::new(),
            ambient_libs_by_profile: DashMap::new(),
            declared_lib_symbols_by_profile: DashMap::new(),
            ambient_lib_symbols_by_profile: DashMap::new(),
            ambient_lib_symbol_sources_by_profile: DashMap::new(),
            well_known_by_profile: DashMap::new(),
            well_known_intrinsics_by_profile: DashMap::new(),
            items: DashMap::new(),
        }
    }

    /// Get the ModuleId for a language item's defining module.
    pub fn module_for_item(&self, item: LanguageSymbol) -> ModuleId {
        *self
            .core_module_by_item
            .get(&item)
            .unwrap_or_else(|| panic!("language item {item:?} not registered"))
    }

    /// Load a lib module set (e.g., "dom", "es2024").
    /// Returns None if the lib name is not registered.
    pub fn load_lib(
        &self,
        name: &str,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        profile_key: &ProfileKey,
    ) -> Option<Vec<ModuleId>> {
        // create a local cycle guard
        let mut loading = HashSet::new();

        // load the lib with cycle tracking
        self.load_lib_inner(name, files, modules, profile_key, &mut loading)
    }

    /// Load a lib module set with cycle tracking.
    fn load_lib_inner(
        &self,
        name: &str,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
        profile_key: &ProfileKey,
        loading: &mut HashSet<String>,
    ) -> Option<Vec<ModuleId>> {
        // return cached modules when available
        if let Some(cached) = self.cached_lib_modules(name, profile_key) {
            return Some(cached);
        }

        // skip recursive cycles in the current load chain
        if loading.contains(name) {
            return Some(Vec::new());
        }

        // load the lib metadata
        let lib = builtin_lib(name)?;

        // check if the lib has any sources for this target
        let runtime = BuiltinRuntime::from(profile_key.runtime);
        let output = BuiltinOutputFormat::from(profile_key.output);
        let platform = BuiltinPlatform::from(profile_key.platform);
        let filtered_sources = lib
            .sources
            .iter()
            .copied()
            .filter(|source| source.matches_target(runtime, output, platform))
            .collect::<Vec<_>>();
        if filtered_sources.is_empty() {
            return None;
        }

        // wait if another thread is already loading this lib
        let loading_key = (profile_key.clone(), name.to_string());
        if self
            .lib_loading_by_name
            .insert(loading_key.clone(), ())
            .is_some()
        {
            return self.wait_for_lib_modules(name, profile_key);
        }

        // track this lib for the current load chain
        loading.insert(name.to_string());

        // recursively load dependencies
        let mut dependencies = Vec::new();
        let mut seen_dependencies = HashSet::new();

        // seed dependencies from lib metadata
        for &dependency in lib.dependencies {
            if seen_dependencies.insert(dependency.to_string()) {
                dependencies.push(dependency.to_string());
            }
        }

        // extend dependencies with reference lib directives
        for reference in lib.reference_libs() {
            if seen_dependencies.insert(reference.to_string()) {
                dependencies.push(reference.to_string());
            }
        }

        // load each dependency in order
        for dependency in dependencies {
            // skip self references
            if dependency == lib.name {
                continue;
            }

            // load each dependency or exit early
            if self
                .load_lib_inner(
                    &dependency,
                    files.clone(),
                    modules.clone(),
                    profile_key,
                    loading,
                )
                .is_none()
            {
                self.lib_loading_by_name.remove(&loading_key);
                loading.remove(name);
                return None;
            }
        }

        // register lib sources
        let mut module_ids = Vec::with_capacity(filtered_sources.len());
        for source in filtered_sources {
            let module_id =
                self.register_lib_source(&source, lib.kind, files.clone(), modules.clone());
            module_ids.push(module_id);
        }

        // register module to lib name mappings
        for module_id in &module_ids {
            self.lib_name_by_module.insert(*module_id, lib.name);
        }

        // cache module ids
        self.lib_module_by_name
            .insert((profile_key.clone(), name.to_string()), module_ids.clone());

        // clear load markers
        self.lib_loading_by_name.remove(&loading_key);
        loading.remove(name);

        Some(module_ids)
    }

    /// Clone cached module ids and refresh lib name mappings.
    fn cached_lib_modules(&self, name: &str, profile_key: &ProfileKey) -> Option<Vec<ModuleId>> {
        // read cached module ids
        let cached = self
            .lib_module_by_name
            .get(&(profile_key.clone(), name.to_string()))?;

        // refresh module to lib name mappings when possible
        if let Some(lib) = builtin_lib(name) {
            for module_id in cached.iter() {
                self.lib_name_by_module.insert(*module_id, lib.name);
            }
        }

        Some(cached.clone())
    }

    /// Wait for a lib that is already loading elsewhere.
    fn wait_for_lib_modules(&self, name: &str, profile_key: &ProfileKey) -> Option<Vec<ModuleId>> {
        loop {
            // return cached modules when they appear
            if let Some(cached) = self.cached_lib_modules(name, profile_key) {
                return Some(cached);
            }

            // stop waiting if the load marker is gone
            if !self
                .lib_loading_by_name
                .contains_key(&(profile_key.clone(), name.to_string()))
            {
                return None;
            }

            // yield to the loader thread
            std::thread::yield_now();
        }
    }

    /// Set the ambient lib modules for a profile key.
    pub fn set_ambient_libs(&self, profile_key: &ProfileKey, modules: Vec<ModuleId>) {
        self.ambient_libs_by_profile
            .insert(profile_key.clone(), modules);
    }

    /// Get the ambient lib modules for a profile key, if any.
    pub fn ambient_libs(&self, profile_key: &ProfileKey) -> Option<Vec<ModuleId>> {
        self.ambient_libs_by_profile
            .get(profile_key)
            .map(|modules| modules.clone())
    }

    /// Get the declared lib symbol group for a profile key and name.
    pub fn get_declared_lib_symbol_group(
        &self,
        profile_key: &ProfileKey,
        name: StringId,
    ) -> Option<SymbolGroup> {
        self.declared_lib_symbols_by_profile
            .get(profile_key)
            .and_then(|symbols| symbols.get(&name).copied())
    }

    /// Get a declared lib symbol for the given space order.
    pub fn get_declared_lib_symbol_from(
        &self,
        profile_key: &ProfileKey,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        self.get_declared_lib_symbol_group(profile_key, name)
            .and_then(|group| group.symbol_for_space_order(order))
    }

    /// Get the lib name for a module, if any.
    pub fn lib_name_for_module(&self, module_id: ModuleId) -> Option<&'static str> {
        self.lib_name_by_module.get(&module_id).map(|name| *name)
    }

    /// Get well-known symbols for a profile key.
    pub fn well_known_symbols(&self, profile_key: &ProfileKey) -> Option<WellKnownSymbols> {
        self.well_known_by_profile
            .get(profile_key)
            .map(|symbols| symbols.clone())
    }

    /// Get well-known intrinsic bindings for a profile key.
    pub fn well_known_intrinsics(&self, profile_key: &ProfileKey) -> Option<WellKnownIntrinsics> {
        self.well_known_intrinsics_by_profile
            .get(profile_key)
            .map(|intrinsics| intrinsics.clone())
    }

    /// Set the declared lib symbols for a profile key.
    pub fn set_declared_lib_symbols(
        &self,
        profile_key: &ProfileKey,
        symbols: IndexMap<StringId, SymbolGroup>,
    ) {
        self.declared_lib_symbols_by_profile
            .insert(profile_key.clone(), symbols);
    }

    /// Set the ambient lib symbols for a profile key.
    pub fn set_ambient_lib_symbols(
        &self,
        profile_key: &ProfileKey,
        symbols: IndexMap<StringId, SymbolGroup>,
    ) {
        self.ambient_lib_symbols_by_profile
            .insert(profile_key.clone(), symbols);
    }

    /// Set the ambient lib symbol sources for a profile key.
    pub fn set_ambient_lib_symbol_sources(
        &self,
        profile_key: &ProfileKey,
        sources: IndexMap<AmbientLibSymbolKey, Vec<GlobalSymbolId>>,
    ) {
        self.ambient_lib_symbol_sources_by_profile
            .insert(profile_key.clone(), sources);
    }

    /// Get ambient lib symbol sources for a profile key, key, and space.
    pub fn get_ambient_lib_symbol_sources(
        &self,
        profile_key: &ProfileKey,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let lookup = AmbientLibSymbolKey { key, space };
        self.ambient_lib_symbol_sources_by_profile
            .get(profile_key)
            .and_then(|sources| sources.get(&lookup).cloned())
    }

    /// Get an ambient lib symbol for the given space order.
    pub fn get_ambient_lib_symbol_for_space_order(
        &self,
        profile_key: &ProfileKey,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        self.ambient_lib_symbols_by_profile
            .get(profile_key)
            .and_then(|symbols| symbols.get(&name).copied())
            .and_then(|group| group.symbol_for_space_order(order))
    }

    /// Set well-known symbols for a profile key.
    pub fn set_well_known_symbols(&self, profile_key: &ProfileKey, symbols: WellKnownSymbols) {
        self.well_known_by_profile
            .insert(profile_key.clone(), symbols);
    }

    /// Set well-known intrinsic bindings for a profile key.
    pub fn set_well_known_intrinsics(
        &self,
        profile_key: &ProfileKey,
        intrinsics: WellKnownIntrinsics,
    ) {
        self.well_known_intrinsics_by_profile
            .insert(profile_key.clone(), intrinsics);
    }

    /// Get builtin modules that define intrinsic bindings.
    pub fn intrinsic_module_ids(&self) -> Vec<ModuleId> {
        // filter core modules to intrinsic paths
        let mut module_ids = Vec::new();
        let mut seen = HashSet::new();
        for (path, module_id) in self.core_module_by_path.iter() {
            if !path.starts_with("intrinsic/") {
                // high-tech filter
                continue;
            }
            if seen.insert(*module_id) {
                module_ids.push(*module_id);
            }
        }
        module_ids
    }

    /// Register a lib source as a module.
    fn register_lib_source(
        &self,
        source: &BuiltinLibSource,
        kind: BuiltinLibKind,
        files: Arc<FileRegistry>,
        modules: Arc<ModuleRegistry>,
    ) -> ModuleId {
        // check if module already exists
        let uri = Uri::from_string(source.virtual_path());
        if let Some(module_id) = modules.get_id_by_uri(&uri) {
            return module_id;
        }

        // select file/language types for the builtin source
        let file_type = file_type_for_builtin_name(source.name);
        let language_type = LanguageType::from(file_type);

        // create file
        let file_id = files.next_id();
        let file = File::from_text(
            file_id,
            source.name.to_string(),
            uri.clone(),
            None,
            file_type,
            source.content.to_string(),
        );
        let file_version = file.version;
        files.insert(file);

        // create module id from path
        let module_path = source.module_path();
        let module_id = ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(&module_path));

        // create blank module (will be parsed/bound later)
        let loader = Loader::from_file_type(file_type);
        let module = Module::blank(
            module_id,
            file_id,
            file_version,
            uri,
            None,
            BUILTIN_PACKAGE_ID,
            None,
            SourceType::Module,
            ModuleFormat::Esm,
            language_type,
            loader,
            ModuleSource::Builtin(kind),
        );
        modules.insert(module);

        module_id
    }
}
