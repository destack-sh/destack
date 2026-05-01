use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::DiskCacheStore;
use destack_source::{FileSystem, PhysicalFileSystem};
use destack_workspace::{HostEnvironment, Repository, Revision, TsConfigDeclaration};

use crate::{
    CachePolicy, Resolution, Resolver, ResolverBase, ResolverContext, ResolverOptions,
    ResolverResult, ResolverSearch, ResolverSource, TypeScriptOptionsReferences,
};

impl Resolver {
    /// Create a test resolver from one file system.
    pub(crate) fn for_test_file_system(fs: Arc<dyn FileSystem>, options: ResolverOptions) -> Self {
        let repository = test_repository(fs);

        Self::new(repository, options, ResolverSource::FileSystem)
    }

    /// Create a test resolver backed by the host file system.
    pub(crate) fn for_tests(options: ResolverOptions) -> Self {
        let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        Self::for_test_file_system(fs, options)
    }

    /// Resolve a specifier from a directory using the test support adapter.
    pub(crate) fn resolve_test_directory<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> ResolverResult<Resolution> {
        let mut ctx = test_resolve_context();
        let search = ResolverSearch::root(&self.options);

        self.resolve_from_base(
            ResolverBase::Directory(directory.as_ref()),
            specifier,
            search,
            &mut ctx,
        )
    }

    /// Resolve a specifier from a concrete base file using the test support adapter.
    pub(crate) fn resolve_test_file<P: AsRef<Path>>(
        &self,
        file: P,
        specifier: &str,
    ) -> ResolverResult<Resolution> {
        let mut ctx = test_resolve_context();
        let search = ResolverSearch::root(&self.options);

        self.resolve_from_base(
            ResolverBase::File(file.as_ref()),
            specifier,
            search,
            &mut ctx,
        )
    }

    /// Resolve a specifier from a directory using the test support adapter and return dependencies.
    pub(crate) fn resolve_test_directory_with_dependencies<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> ResolverResult<(Resolution, Vec<destack_artifact::ArtifactDependency>)> {
        let mut ctx = test_resolve_context();
        let resolution = self.resolve_from_base(
            ResolverBase::Directory(directory.as_ref()),
            specifier,
            ResolverSearch::root(&self.options),
            &mut ctx,
        )?;

        Ok((resolution, ctx.into_dependencies()))
    }

    /// Find the effective tsconfig.json for one file path using the test support adapter.
    pub(crate) fn find_test_tsconfig_for_file(
        &self,
        path: &Path,
    ) -> ResolverResult<Option<TsConfigDeclaration>> {
        let mut ctx = test_resolve_context();
        self.validate_file_base(path, &mut ctx)?;
        self.find_applicable_tsconfig(ResolverBase::File(path), path, &mut ctx)
    }

    /// Find the effective tsconfig.json for one directory path using the test support adapter.
    pub(crate) fn find_test_tsconfig_for_directory(
        &self,
        path: &Path,
    ) -> ResolverResult<Option<TsConfigDeclaration>> {
        let mut ctx = test_resolve_context();
        self.validate_directory_base(path, &mut ctx)?;
        self.find_applicable_tsconfig(ResolverBase::Directory(path), path, &mut ctx)
    }

    /// Resolve a `tsconfig.json` file at the given path through the test support adapter.
    pub(crate) fn resolve_tsconfig<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> ResolverResult<TsConfigDeclaration> {
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
pub(crate) fn test_resolve_context() -> ResolverContext {
    ResolverContext::new(Revision::NULL)
}
