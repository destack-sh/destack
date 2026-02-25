use std::fmt;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

use destack_source::{FileRegistry, FileSystem, PhysicalFileSystem};
use destack_workspace::{PackageRegistry, Program, Session, TsConfigRegistry};

use crate::ResolveOptions;

/// Shared resolver runtime state reused across option variants.
#[derive(Debug)]
pub(crate) struct ResolverState {
    /// Cached Yarn PnP manifest for this resolver state.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pnp_manifest: OnceLock<pnp::Manifest>,
    /// Cached opened zip archives for Yarn PnP zip path reads.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pnp_lru: pnp::fs::LruZipCache<Vec<u8>>,
}

impl ResolverState {
    /// Create a new resolver state.
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pnp_manifest: OnceLock::new(),
            #[cfg(not(target_arch = "wasm32"))]
            pnp_lru: pnp::fs::LruZipCache::new(50, pnp::fs::open_zip_via_read_p),
        }
    }
}

/// Module resolver implementing Node.js-style resolution.
pub struct Resolver {
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The file registry.
    pub files: Arc<FileRegistry>,
    /// The package registry.
    pub packages: Arc<PackageRegistry>,
    /// The tsconfig registry.
    pub tsconfigs: Arc<TsConfigRegistry>,
    /// Configuration options controlling resolution behavior.
    pub options: ResolveOptions,
    /// Shared runtime state for resolver caches.
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
        Self {
            fs,
            files,
            packages,
            tsconfigs,
            options,
            state: Arc::new(ResolverState::new()),
        }
    }

    /// Create a new resolver from a Program (unpacks its registries).
    pub fn from_program(program: &Program, options: ResolveOptions) -> Self {
        Self {
            fs: program.fs.clone(),
            files: program.files.clone(),
            packages: program.packages.clone(),
            tsconfigs: program.tsconfigs.clone(),
            options,
            state: Arc::new(ResolverState::new()),
        }
    }

    /// Create a new resolver from a Session (unpacks its registries).
    pub fn from_session(session: &Session, options: ResolveOptions) -> Self {
        Self {
            fs: session.fs.clone(),
            files: session.files.clone(),
            packages: session.packages.clone(),
            tsconfigs: session.tsconfigs.clone(),
            options,
            state: Arc::new(ResolverState::new()),
        }
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

        Self {
            fs: self.fs.clone(),
            files: self.files.clone(),
            packages: self.packages.clone(),
            tsconfigs: self.tsconfigs.clone(),
            options,
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
