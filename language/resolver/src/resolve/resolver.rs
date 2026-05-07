use std::fmt;
use std::sync::Arc;

use destack_source::FileSystem;
use destack_workspace::{Repository, Revision};

use crate::{AliasTable, MountTable, ResolverContext, ResolverOptions};

/// The source truth used for resolver path reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolverSource {
    /// Read from the active immutable repository revision.
    Revision,
    /// Read from the repository file system.
    FileSystem,
}

/// The resolver implementing Destack module identity.
pub struct Resolver {
    /// The repository source world for resolution.
    pub(crate) repository: Arc<Repository>,
    /// The file system used for reads and metadata lookups.
    fs: Arc<dyn FileSystem>,
    /// The source truth used for path reads.
    pub(crate) source: ResolverSource,
    /// The configuration options controlling resolution behavior.
    pub(crate) options: ResolverOptions,
    /// The explicit alias matchers for this option set.
    pub(crate) aliases: AliasTable,
    /// The mounted package source roots for this option set.
    pub(crate) mounts: MountTable,
}

impl fmt::Debug for Resolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.options.fmt(f)
    }
}

impl Resolver {
    /// Create a resolver from explicit repository, options, and source truth.
    pub(crate) fn new(
        repository: Arc<Repository>,
        options: ResolverOptions,
        source: ResolverSource,
    ) -> Self {
        let aliases = AliasTable::new(&options.alias);
        let mounts = MountTable::new(&options.mounts);

        Self {
            fs: Arc::clone(repository.file_system()),
            repository,
            source,
            options,
            aliases,
            mounts,
        }
    }

    /// Create a resolver from one repository revision source.
    pub fn from_repository(repository: Arc<Repository>, options: ResolverOptions) -> Self {
        Self::new(repository, options, ResolverSource::Revision)
    }

    /// Clone the resolver with new options.
    pub fn with_options(&self, options: ResolverOptions) -> Self {
        Self::new(self.repository.clone(), options, self.source)
    }

    /// Get the backing repository.
    #[inline]
    pub fn repository(&self) -> &Arc<Repository> {
        &self.repository
    }

    /// Return the active resolver options.
    #[inline]
    pub fn options(&self) -> &ResolverOptions {
        &self.options
    }

    /// Get a reference to the file system.
    #[inline]
    pub(crate) fn fs(&self) -> &dyn FileSystem {
        self.fs.as_ref()
    }

    /// Return the repository and active revision for one request context.
    pub(crate) fn repository_revision(&self, ctx: &ResolverContext) -> (&Repository, Revision) {
        (self.repository.as_ref(), ctx.revision())
    }
}
