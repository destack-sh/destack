use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_source::{FileSystem, MemoryFileSystem};
use destack_workspace::{HostEnvironment, Repository, Revision};

use crate::{
    Resolution, Resolver, ResolverBase, ResolverContext, ResolverOptions, ResolverResult,
    ResolverSearch, ResolverSource,
};

#[cfg(target_os = "windows")]
const PROJECT_ROOT: &str = "C:/project";

#[cfg(not(target_os = "windows"))]
const PROJECT_ROOT: &str = "/project";

/// Create a resolver backed by one in-memory project.
pub(crate) fn resolver(files: &[(&str, &str)], options: ResolverOptions) -> Resolver {
    let fs = MemoryFileSystem::new();

    for (file, content) in files {
        fs.add_file(project(file), content.as_bytes())
            .expect("failed to add resolver test file");
    }

    Resolver::for_test_file_system(Arc::new(fs), options)
}

/// Return one absolute test project path.
pub(crate) fn project(suffix: &str) -> PathBuf {
    let suffix = suffix.strip_prefix('/').unwrap_or(suffix);

    PathBuf::from(PROJECT_ROOT).join(suffix)
}

impl Resolver {
    /// Create a test resolver from one file system.
    pub(crate) fn for_test_file_system(fs: Arc<dyn FileSystem>, options: ResolverOptions) -> Self {
        let repository = test_repository(fs);

        Self::new(repository, options, ResolverSource::FileSystem)
    }

    /// Resolve a specifier from a concrete base file using the test support adapter.
    pub(crate) fn resolve_test_file<P: AsRef<std::path::Path>>(
        &self,
        file: P,
        specifier: &str,
    ) -> ResolverResult<Resolution> {
        let mut ctx = test_resolve_context();
        let search = ResolverSearch::root();

        self.resolve_from_base(
            ResolverBase::File(file.as_ref()),
            specifier,
            search,
            &mut ctx,
        )
    }

    /// Resolve a specifier from a concrete base directory using the test support adapter.
    pub(crate) fn resolve_test_directory<P: AsRef<std::path::Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> ResolverResult<Resolution> {
        let mut ctx = test_resolve_context();
        let search = ResolverSearch::root();

        self.resolve_from_base(
            ResolverBase::Directory(directory.as_ref()),
            specifier,
            search,
            &mut ctx,
        )
    }
}

/// Create one synthetic repository for host backed tests.
fn test_repository(fs: Arc<dyn FileSystem>) -> Arc<Repository> {
    Arc::new(Repository::new(
        test_repository_root(),
        Arc::new(DiskCacheStore::new()),
        fs,
        HostEnvironment::capture_process(),
    ))
}

/// Build one synthetic repository root for host backed tests.
fn test_repository_root() -> PathBuf {
    std::env::temp_dir().join(format!("destack-resolver-test-{}", std::process::id()))
}

/// Build one request local test support context.
pub(crate) fn test_resolve_context() -> ResolverContext {
    ResolverContext::new(Revision::NULL)
}
