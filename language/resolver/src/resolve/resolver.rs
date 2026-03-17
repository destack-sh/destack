use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use std::sync::{Arc, RwLock};

use destack_source::{FileRegistry, FileSystem, PackageId, PhysicalFileSystem};
use destack_workspace::{PackageRegistry, Program, Session, TsConfigId, TsConfigRegistry};

use crate::{CompiledAliasTable, ResolveOptions, ResolveOrigin};

/// The shared runtime state reused across option variants.
#[derive(Debug)]
pub(crate) struct ResolverState {
    /// The cached nearest package scope lookup results.
    pub(crate) package_scope_cache: RwLock<HashMap<PathBuf, Option<PackageId>>>,
    /// The cached nearest tsconfig lookup results.
    pub(crate) nearest_tsconfig_cache: RwLock<HashMap<PathBuf, Option<TsConfigId>>>,
    /// The cached effective file tsconfig lookup results.
    pub(crate) effective_file_tsconfig_cache: RwLock<HashMap<PathBuf, Option<TsConfigId>>>,
    /// The cached effective directory tsconfig lookup results.
    pub(crate) effective_directory_tsconfig_cache: RwLock<HashMap<PathBuf, Option<TsConfigId>>>,
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
    pub files: Arc<FileRegistry>,
    /// The package registry for parsed package manifests.
    pub packages: Arc<PackageRegistry>,
    /// The tsconfig registry for parsed TypeScript projects.
    pub tsconfigs: Arc<TsConfigRegistry>,
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
    pub fn new(
        fs: Arc<dyn FileSystem>,
        files: Arc<FileRegistry>,
        packages: Arc<PackageRegistry>,
        tsconfigs: Arc<TsConfigRegistry>,
        options: ResolveOptions,
    ) -> Self {
        let compiled_alias = CompiledAliasTable::from_aliases(&options.alias);
        let compiled_fallback = CompiledAliasTable::from_aliases(&options.fallback);

        Self {
            fs,
            files,
            packages,
            tsconfigs,
            options,
            compiled_alias,
            compiled_fallback,
            state: Arc::new(ResolverState::new()),
        }
    }

    /// Create a new resolver from a Program (unpacks its registries).
    pub fn from_program(program: &Program, options: ResolveOptions) -> Self {
        Self::new(
            program.fs.clone(),
            program.files.clone(),
            program.packages.clone(),
            program.tsconfigs.clone(),
            options,
        )
    }

    /// Create a new resolver from a Session (unpacks its registries).
    pub fn from_session(session: &Session, options: ResolveOptions) -> Self {
        Self::new(
            session.fs.clone(),
            session.files.clone(),
            session.packages.clone(),
            session.tsconfigs.clone(),
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
            tsconfigs: self.tsconfigs.clone(),
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
    pub(crate) fn cached_nearest_tsconfig(&self, path: &Path) -> Option<Option<TsConfigId>> {
        self.state
            .nearest_tsconfig_cache
            .read()
            .unwrap()
            .get(path)
            .cloned()
    }

    /// Store one cached nearest tsconfig lookup result.
    pub(crate) fn cache_nearest_tsconfig(&self, path: &Path, tsconfig_id: Option<TsConfigId>) {
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
    ) -> Option<Option<TsConfigId>> {
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
        tsconfig_id: Option<TsConfigId>,
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
        let files = Arc::new(FileRegistry::new());
        let packages = Arc::new(PackageRegistry::new());
        let tsconfigs = Arc::new(TsConfigRegistry::new());
        Self::new(fs, files, packages, tsconfigs, options)
    }

    /// Create a new resolver with a custom file system and empty registries (for testing).
    pub(crate) fn blank(fs: Arc<dyn FileSystem>, options: ResolveOptions) -> Self {
        let files = Arc::new(FileRegistry::new());
        let packages = Arc::new(PackageRegistry::new());
        let tsconfigs = Arc::new(TsConfigRegistry::new());
        Self::new(fs, files, packages, tsconfigs, options)
    }
}
