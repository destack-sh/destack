use std::fmt;
use std::sync::Arc;

use destack_source::FileSystem;
use destack_workspace::{Package, PackageDeclaration, Repository, Revision};

use crate::{CompiledAliasTable, ResolveContext, ResolveOptions};

/// One discovered package scope for one resolve query.
#[derive(Debug, Clone)]
pub(crate) struct PackageScope {
    /// The semantic package view.
    pub package: Package,
    /// The parsed package declaration when present.
    pub package_declaration: Option<PackageDeclaration>,
}

/// The resolver implementing Node.js style module resolution.
pub struct Resolver {
    /// The repository source world for resolution.
    pub(crate) repository: Arc<Repository>,
    /// The file system used for reads and metadata lookups.
    fs: Arc<dyn FileSystem>,
    /// The configuration options controlling resolution behavior.
    pub options: ResolveOptions,
    /// The precompiled primary alias matchers for this option set.
    pub(crate) compiled_alias: CompiledAliasTable,
    /// The precompiled fallback alias matchers for this option set.
    pub(crate) compiled_fallback: CompiledAliasTable,
}

impl fmt::Debug for Resolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.options.fmt(f)
    }
}

impl Resolver {
    /// Create a resolver from explicit parts.
    pub(crate) fn from_parts(repository: Arc<Repository>, options: ResolveOptions) -> Self {
        let compiled_alias = CompiledAliasTable::from_aliases(&options.alias);
        let compiled_fallback = CompiledAliasTable::from_aliases(&options.fallback);

        Self {
            fs: Arc::clone(repository.file_system()),
            repository,
            options,
            compiled_alias,
            compiled_fallback,
        }
    }

    /// Create a resolver from one repository.
    pub fn from_repository(repository: Arc<Repository>, options: ResolveOptions) -> Self {
        Self::from_parts(repository, options)
    }

    /// Clone the resolver with new options.
    pub fn with_options(&self, options: ResolveOptions) -> Self {
        Self::from_parts(self.repository.clone(), options)
    }

    /// Get the backing repository.
    #[inline]
    pub fn repository(&self) -> &Arc<Repository> {
        &self.repository
    }

    /// Get a reference to the file system.
    #[inline]
    pub(crate) fn fs(&self) -> &dyn FileSystem {
        self.fs.as_ref()
    }

    /// Return the active repository revision for one request context.
    pub(crate) fn source_world(&self, ctx: &ResolveContext) -> (&Repository, Revision) {
        (self.repository().as_ref(), ctx.revision())
    }
}
