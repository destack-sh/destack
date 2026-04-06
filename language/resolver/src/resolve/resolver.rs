use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use std::sync::{Arc, RwLock};

use destack_source::{FileId, FileStore, FileSystem, PackageId, PhysicalFileSystem};
use destack_workspace::{
    DestackDeclaration, Package, PackageDeclaration, PackageOptions, Repository,
    TsConfigDeclaration,
};

use crate::{CompiledAliasTable, ResolveOptions, ResolveOrigin};

/// Private cached resolver package state.
#[derive(Debug, Clone)]
pub(crate) struct ResolverPackageEntry {
    /// The semantic package view.
    pub package: Package,
    /// The parsed package.json declaration when present.
    pub package_declaration: Option<PackageDeclaration>,
    /// The parsed destack.json declaration when present.
    pub destack_declaration: Option<DestackDeclaration>,
    /// The effective package options when present.
    pub package_options: Option<PackageOptions>,
}

/// Private cache of parsed package state for one resolver.
#[derive(Debug, Default)]
pub(crate) struct ResolverPackageCache {
    /// The cached packages by id.
    packages_by_id: RwLock<HashMap<PackageId, Arc<RwLock<ResolverPackageEntry>>>>,
    /// The cached package ids by root path.
    package_ids_by_path: RwLock<HashMap<PathBuf, PackageId>>,
}

impl ResolverPackageCache {
    /// Create one empty resolver package cache.
    fn new() -> Self {
        Self::default()
    }

    /// Get one cached package id by path.
    pub(crate) fn get_id_by_path(&self, path: &Path) -> Option<PackageId> {
        self.package_ids_by_path.read().unwrap().get(path).copied()
    }

    /// Get one cached package by id.
    pub(crate) fn get(&self, id: PackageId) -> Arc<RwLock<ResolverPackageEntry>> {
        self.packages_by_id
            .read()
            .unwrap()
            .get(&id)
            .unwrap_or_else(|| panic!("package not found for id: {id:?}"))
            .clone()
    }

    /// Get one cached package by id when present.
    pub(crate) fn get_maybe(&self, id: PackageId) -> Option<Arc<RwLock<ResolverPackageEntry>>> {
        self.packages_by_id.read().unwrap().get(&id).cloned()
    }

    /// Insert or replace one cached package.
    pub(crate) fn insert(&self, entry: ResolverPackageEntry) {
        let package_id = entry.package.id;
        let package_path = entry.package.path.clone();
        let entry = Arc::new(RwLock::new(entry));

        self.packages_by_id
            .write()
            .unwrap()
            .insert(package_id, entry);

        if let Some(package_path) = package_path {
            self.package_ids_by_path
                .write()
                .unwrap()
                .insert(package_path, package_id);
        }
    }
}

/// The shared runtime state reused across option variants.
#[derive(Debug)]
pub(crate) struct ResolverState {
    /// The cached nearest package scope lookup results.
    pub(crate) package_scope_cache: RwLock<HashMap<PathBuf, Option<PackageId>>>,
    /// The cached nearest tsconfig lookup results.
    pub(crate) nearest_tsconfig_cache: RwLock<HashMap<PathBuf, Option<FileId>>>,
    /// The cached effective file tsconfig lookup results.
    pub(crate) effective_file_tsconfig_cache: RwLock<HashMap<PathBuf, Option<FileId>>>,
    /// The cached effective directory tsconfig lookup results.
    pub(crate) effective_directory_tsconfig_cache: RwLock<HashMap<PathBuf, Option<FileId>>>,
    /// The cached parsed tsconfigs by file id.
    pub(crate) tsconfigs_by_file_id: RwLock<HashMap<FileId, TsConfigDeclaration>>,
    /// The cached tsconfig file ids by path.
    pub(crate) tsconfig_file_ids_by_path: RwLock<HashMap<PathBuf, FileId>>,
    /// The cached Yarn PnP manifest for this resolver state.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pnp_manifest: OnceLock<pnp::Manifest>,
    /// The cached opened zip archives for Yarn PnP zip path reads.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pnp_lru: pnp::fs::LruZipCache<Vec<u8>>,
}

impl ResolverState {
    /// Create a new resolver state.
    pub(crate) fn new() -> Self {
        Self {
            package_scope_cache: RwLock::new(HashMap::new()),
            nearest_tsconfig_cache: RwLock::new(HashMap::new()),
            effective_file_tsconfig_cache: RwLock::new(HashMap::new()),
            effective_directory_tsconfig_cache: RwLock::new(HashMap::new()),
            tsconfigs_by_file_id: RwLock::new(HashMap::new()),
            tsconfig_file_ids_by_path: RwLock::new(HashMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            pnp_manifest: OnceLock::new(),
            #[cfg(not(target_arch = "wasm32"))]
            pnp_lru: pnp::fs::LruZipCache::new(50, pnp::fs::open_zip_via_read_p),
        }
    }
}

/// The resolver implementing Node.js style module resolution.
pub struct Resolver {
    /// The file system used for reads and metadata lookups.
    pub fs: Arc<dyn FileSystem>,
    /// The file registry for parsed source and config files.
    pub files: Arc<FileStore>,
    /// The private cache for parsed package manifests.
    pub(crate) packages: Arc<ResolverPackageCache>,
    /// The configuration options controlling resolution behavior.
    pub options: ResolveOptions,
    /// The precompiled primary alias matchers for this option set.
    pub(crate) compiled_alias: CompiledAliasTable,
    /// The precompiled fallback alias matchers for this option set.
    pub(crate) compiled_fallback: CompiledAliasTable,
    /// The shared runtime state for resolver caches.
    pub(crate) state: Arc<ResolverState>,
}

impl fmt::Debug for Resolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.options.fmt(f)
    }
}

#[allow(dead_code)]
impl Resolver {
    /// Create a new resolver with individual registries.
    pub fn new(fs: Arc<dyn FileSystem>, files: Arc<FileStore>, options: ResolveOptions) -> Self {
        let compiled_alias = CompiledAliasTable::from_aliases(&options.alias);
        let compiled_fallback = CompiledAliasTable::from_aliases(&options.fallback);

        Self {
            fs,
            files,
            packages: Arc::new(ResolverPackageCache::new()),
            options,
            compiled_alias,
            compiled_fallback,
            state: Arc::new(ResolverState::new()),
        }
    }

    /// Create a resolver from one repository.
    pub fn from_repository(repository: &Repository, options: ResolveOptions) -> Self {
        Self::new(
            Arc::clone(repository.file_system()),
            Arc::new(FileStore::new()),
            options,
        )
    }

    /// Clone the resolver with new options.
    pub fn with_options(&self, options: ResolveOptions) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let recreate_pnp_cache = options.yarn_pnp != self.options.yarn_pnp;

        // recreate shared state only when pnp mode changes
        #[cfg(not(target_arch = "wasm32"))]
        let state = if recreate_pnp_cache {
            Arc::new(ResolverState::new())
        } else {
            self.state.clone()
        };
        #[cfg(target_arch = "wasm32")]
        let state = self.state.clone();

        let compiled_alias = CompiledAliasTable::from_aliases(&options.alias);
        let compiled_fallback = CompiledAliasTable::from_aliases(&options.fallback);

        Self {
            fs: self.fs.clone(),
            files: self.files.clone(),
            packages: self.packages.clone(),
            options,
            compiled_alias,
            compiled_fallback,
            state,
        }
    }

    /// Check if two resolvers share one PnP cache.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn shares_pnp_cache_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }

    /// Get a reference to the file system.
    #[inline]
    pub fn fs(&self) -> &dyn FileSystem {
        self.fs.as_ref()
    }

    /// Clear the nearest lookup caches shared by this resolver state.
    pub fn clear_lookup_caches(&self) {
        self.state.package_scope_cache.write().unwrap().clear();
        self.state.nearest_tsconfig_cache.write().unwrap().clear();
        self.state
            .effective_file_tsconfig_cache
            .write()
            .unwrap()
            .clear();
        self.state
            .effective_directory_tsconfig_cache
            .write()
            .unwrap()
            .clear();
    }

    /// Return one cached package snapshot when present.
    pub fn package_maybe(&self, id: PackageId) -> Option<Package> {
        let package = self.packages.get_maybe(id)?;
        let package = package.read().unwrap();

        Some(package.package.clone())
    }

    /// Read one cached nearest package scope result.
    pub(crate) fn cached_package_scope(&self, path: &Path) -> Option<Option<PackageId>> {
        self.state
            .package_scope_cache
            .read()
            .unwrap()
            .get(path)
            .cloned()
    }

    /// Store one cached nearest package scope result.
    pub(crate) fn cache_package_scope(&self, path: &Path, package_id: Option<PackageId>) {
        self.state
            .package_scope_cache
            .write()
            .unwrap()
            .insert(path.to_path_buf(), package_id);
    }

    /// Read one cached nearest tsconfig lookup result.
    pub(crate) fn cached_nearest_tsconfig(&self, path: &Path) -> Option<Option<FileId>> {
        self.state
            .nearest_tsconfig_cache
            .read()
            .unwrap()
            .get(path)
            .cloned()
    }

    /// Store one cached nearest tsconfig lookup result.
    pub(crate) fn cache_nearest_tsconfig(&self, path: &Path, tsconfig_id: Option<FileId>) {
        self.state
            .nearest_tsconfig_cache
            .write()
            .unwrap()
            .insert(path.to_path_buf(), tsconfig_id);
    }

    /// Read one cached effective tsconfig lookup result.
    pub(crate) fn cached_effective_tsconfig(
        &self,
        origin: ResolveOrigin,
        path: &Path,
    ) -> Option<Option<FileId>> {
        match origin {
            ResolveOrigin::File => self
                .state
                .effective_file_tsconfig_cache
                .read()
                .unwrap()
                .get(path)
                .cloned(),
            ResolveOrigin::Directory => self
                .state
                .effective_directory_tsconfig_cache
                .read()
                .unwrap()
                .get(path)
                .cloned(),
        }
    }

    /// Store one cached effective tsconfig lookup result.
    pub(crate) fn cache_effective_tsconfig(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        tsconfig_id: Option<FileId>,
    ) {
        match origin {
            ResolveOrigin::File => {
                self.state
                    .effective_file_tsconfig_cache
                    .write()
                    .unwrap()
                    .insert(path.to_path_buf(), tsconfig_id);
            }
            ResolveOrigin::Directory => {
                self.state
                    .effective_directory_tsconfig_cache
                    .write()
                    .unwrap()
                    .insert(path.to_path_buf(), tsconfig_id);
            }
        }
    }

    /// Create a new resolver with physical file system and empty registries (for testing).
    pub(crate) fn physical(options: ResolveOptions) -> Self {
        let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let files = Arc::new(FileStore::new());
        Self::new(fs, files, options)
    }

    /// Create a new resolver with a custom file system and empty registries (for testing).
    pub(crate) fn blank(fs: Arc<dyn FileSystem>, options: ResolveOptions) -> Self {
        let files = Arc::new(FileStore::new());
        Self::new(fs, files, options)
    }

    /// Return one cached tsconfig by file id.
    pub(crate) fn cached_tsconfig(&self, file_id: FileId) -> Option<TsConfigDeclaration> {
        self.state
            .tsconfigs_by_file_id
            .read()
            .unwrap()
            .get(&file_id)
            .cloned()
    }

    /// Return one cached tsconfig file id by path.
    pub(crate) fn cached_tsconfig_file_id_by_path(&self, path: &Path) -> Option<FileId> {
        self.state
            .tsconfig_file_ids_by_path
            .read()
            .unwrap()
            .get(path)
            .copied()
    }

    /// Store one parsed tsconfig in shared resolver state.
    pub(crate) fn cache_tsconfig(&self, tsconfig: TsConfigDeclaration) {
        let file_id = tsconfig.file_id;
        let path = tsconfig.path.clone();

        self.state
            .tsconfigs_by_file_id
            .write()
            .unwrap()
            .insert(file_id, tsconfig);
        self.state
            .tsconfig_file_ids_by_path
            .write()
            .unwrap()
            .insert(path, file_id);
    }
}
