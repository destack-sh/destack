use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_source::{FileSystem, PhysicalFileSystem};
use destack_workspace::{HostEnvironment, Repository, Revision, TsConfigDeclaration};

use crate::{
    CachePolicy, Resolution, ResolveContext, ResolveError, ResolveOptions, ResolveOrigin,
    ResolveState, ResolveTrace, Resolver, TypeScriptOptionsReferences,
};

impl Resolver {
    /// Create a test resolver from one file system.
    pub(crate) fn for_test_file_system(fs: Arc<dyn FileSystem>, options: ResolveOptions) -> Self {
        let repository = test_repository(fs);

        Self::from_parts(repository, options)
    }

    /// Create a test resolver backed by the host file system.
    pub(crate) fn for_tests(options: ResolveOptions) -> Self {
        let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        Self::for_test_file_system(fs, options)
    }

    /// Resolve a specifier from a directory using the test support adapter.
    pub(crate) fn resolve_test_directory<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = test_resolve_context();
        let state = ResolveState::new(&self.options);

        self.resolve_query_path(
            ResolveOrigin::Directory,
            directory.as_ref(),
            specifier,
            state,
            &mut ctx,
        )
    }

    /// Resolve a specifier from a concrete issuer file using the test support adapter.
    pub(crate) fn resolve_test_file<P: AsRef<Path>>(
        &self,
        file: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = test_resolve_context();
        let state = ResolveState::new(&self.options);

        self.resolve_query_path(
            ResolveOrigin::File,
            file.as_ref(),
            specifier,
            state,
            &mut ctx,
        )
    }

    /// Resolve a specifier from a directory using the test support adapter while collecting dependency tracing.
    pub(crate) fn resolve_test_directory_with_trace<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
        trace: &mut ResolveTrace,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = test_resolve_trace_context();
        let resolution = self.resolve_query_path(
            ResolveOrigin::Directory,
            directory.as_ref(),
            specifier,
            ResolveState::new(&self.options),
            &mut ctx,
        );

        ctx.append_trace_to(trace);

        resolution
    }

    /// Find the effective tsconfig.json for one file path using the test support adapter.
    pub(crate) fn find_test_tsconfig_for_file(
        &self,
        path: &Path,
    ) -> Result<Option<TsConfigDeclaration>, ResolveError> {
        let mut ctx = test_resolve_context();
        self.validate_file_origin(path, &mut ctx)?;
        self.find_applicable_tsconfig(ResolveOrigin::File, path, &mut ctx)
    }

    /// Find the effective tsconfig.json for one directory path using the test support adapter.
    pub(crate) fn find_test_tsconfig_for_directory(
        &self,
        path: &Path,
    ) -> Result<Option<TsConfigDeclaration>, ResolveError> {
        let mut ctx = test_resolve_context();
        self.validate_directory_origin(path, &mut ctx)?;
        self.find_applicable_tsconfig(ResolveOrigin::Directory, path, &mut ctx)
    }

    /// Resolve a `tsconfig.json` file at the given path through the test support adapter.
    pub(crate) fn resolve_tsconfig<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<TsConfigDeclaration, ResolveError> {
        let mut ctx = test_resolve_context();
        let tsconfig_path = self.materialize_tsconfig_path(path.as_ref(), &mut ctx)?;
        self.read_tsconfig(
            true,
            &tsconfig_path,
            &TypeScriptOptionsReferences::Automatic,
            &mut ctx,
            CachePolicy::UseCache,
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
pub(crate) fn test_resolve_context() -> ResolveContext {
    ResolveContext::new(Revision::NULL)
}

/// Build one tracing test support context.
pub(crate) fn test_resolve_trace_context() -> ResolveContext {
    ResolveContext::with_trace(Revision::NULL)
}
