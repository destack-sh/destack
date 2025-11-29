use std::path::Path;

use destack_dir::{PackageId, TsConfigId};

use crate::{Resolution, ResolveContext, ResolveError, Resolver};

impl Resolver {
    /// Resolve a specifier from a directory to a file path.
    #[tracing::instrument(
        name = "resolver.resolve",
        level = "debug",
        skip(self, directory),
        fields(specifier)
    )]
    pub fn resolve<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        let directory = directory.as_ref();
        tracing::debug!(?directory, ?specifier, "resolver.resolve");
        self.resolve_in_context(directory, specifier, &mut ResolveContext::default())
    }

    /// Resolve a specifier with a custom context for dependency tracking.
    pub fn resolve_with_context<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        // track dependencies in a new context
        let mut inner_ctx = ResolveContext::default();
        inner_ctx.found_dependencies.replace(vec![]);
        inner_ctx.missing_dependencies.replace(vec![]);

        // resolve
        let resolution = self.resolve_in_context(directory.as_ref(), specifier, &mut inner_ctx);

        // append dependencies to caller's context
        if let Some(deps) = &mut inner_ctx.found_dependencies {
            ctx.found_dependencies
                .get_or_insert_with(Vec::new)
                .append(deps);
        }
        if let Some(deps) = &mut inner_ctx.missing_dependencies {
            ctx.missing_dependencies
                .get_or_insert_with(Vec::new)
                .append(deps);
        }

        resolution
    }

    /// Perform the resolution with a mutable context.
    fn resolve_in_context(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        ctx.is_fully_specified = self.options.is_fully_specified;

        let resolved_path = self.require(path, specifier, ctx)?;
        let path = self.load_realpath(&resolved_path)?;

        Ok(Resolution {
            path,
            query: ctx.query.take(),
            fragment: ctx.fragment.take(),
        })
    }

    /// Find the nearest package.json by walking up parent directories.
    #[tracing::instrument(name = "resolver.resolve.package", level = "debug", fields(path))]
    pub fn find_package(&self, path: &Path) -> Option<PackageId> {
        tracing::debug!(?path, "resolver.resolve.package");
        let mut ctx = ResolveContext::default();
        let package_id = self.find_package_json(path, &mut ctx).ok()??;
        Some(package_id)
    }

    /// Find the nearest tsconfig.json by walking up parent directories.
    #[tracing::instrument(name = "resolver.resolve.tsconfig", level = "debug", fields(path))]
    pub fn find_tsconfig(&self, path: &Path) -> Option<TsConfigId> {
        tracing::debug!(?path, "resolver.resolve.tsconfig");
        let mut ctx = ResolveContext::default();
        let tsconfig_id = self.find_tsconfig_json(path, &mut ctx).ok()??;
        Some(tsconfig_id)
    }
}
