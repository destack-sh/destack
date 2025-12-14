use std::fmt;
use std::sync::Arc;

use destack_source::{FileRegistry, FileSystem, PhysicalFileSystem};
use destack_workspace::{PackageRegistry, Program, TsConfigRegistry};

use crate::ResolveOptions;

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
        }
    }

    /// Clone the resolver with new options.
    pub fn with_options(&self, options: ResolveOptions) -> Self {
        Self {
            fs: self.fs.clone(),
            files: self.files.clone(),
            packages: self.packages.clone(),
            tsconfigs: self.tsconfigs.clone(),
            options,
        }
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
