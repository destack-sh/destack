use std::path::Path;

use dyst_dir::{PackageId, TsConfigId};

use crate::{Resolution, ResolutionContext, ResolveError, Resolver};

impl Resolver {
    /// Resolve a specifier from a directory to a file path.
    pub fn resolve<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        self.resolve_in_context(
            directory.as_ref(),
            specifier,
            &mut ResolutionContext::default(),
        )
    }

    /// Resolve a specifier with a custom context for dependency tracking.
    pub fn resolve_with_context<P: AsRef<Path>>(
        &self,
        directory: P,
        specifier: &str,
        ctx: &mut ResolutionContext,
    ) -> Result<Resolution, ResolveError> {
        // track dependencies in a new context
        let mut inner_ctx = ResolutionContext::default();
        inner_ctx.found_dependencies.replace(vec![]);
        inner_ctx.missing_dependencies.replace(vec![]);

        let result = self.resolve_in_context(directory.as_ref(), specifier, &mut inner_ctx);

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

        result
    }

    /// Perform the resolution with a mutable context.
    fn resolve_in_context(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolutionContext,
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
    pub fn find_package(&self, path: &Path) -> Option<PackageId> {
        let mut ctx = ResolutionContext::default();
        let package_id = self.find_package_json_id(path, &mut ctx).ok()??;
        Some(package_id)
    }

    /// Find the nearest tsconfig.json by walking up parent directories.
    pub fn find_tsconfig_id(&self, path: &Path) -> Option<TsConfigId> {
        let mut ctx = ResolutionContext::default();
        let tsconfig_id = self.find_tsconfig(path, &mut ctx).ok()??;
        Some(tsconfig_id)
    }
}
