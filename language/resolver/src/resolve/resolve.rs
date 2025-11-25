use std::path::Path;

use dyst_dir::{DsConfigId, PackageId, TsConfigId};

use crate::{Resolution, ResolutionContext, ResolveError, Resolver};

impl Resolver {
    /// Resolve a specifier from a directory to a file path.
    ///
    /// This is the main entry point for module resolution. It handles all specifier types:
    /// - Relative paths (`./foo`, `../bar`)
    /// - Absolute paths (`/foo`, `C:\foo`)
    /// - Bare specifiers (`lodash`, `@scope/pkg`)
    /// - Internal imports (`#internal`)
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
        resolve_context: &mut ResolutionContext,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = ResolutionContext::default();
        ctx.found_dependencies.replace(vec![]);
        ctx.missing_dependencies.replace(vec![]);

        let result = self.resolve_in_context(directory.as_ref(), specifier, &mut ctx);

        // append dependencies to caller's context
        if let Some(deps) = &mut ctx.found_dependencies {
            resolve_context
                .found_dependencies
                .get_or_insert_with(Vec::new)
                .append(deps);
        }
        if let Some(deps) = &mut ctx.missing_dependencies {
            resolve_context
                .missing_dependencies
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
    ///
    /// Returns the PackageId if found, or None if no package.json exists in any parent.
    pub fn find_package(&self, path: &Path) -> Option<PackageId> {
        let mut ctx = ResolutionContext::default();
        let package_id = self.find_package_json_id(path, &mut ctx).ok()??;
        Some(package_id)
    }

    /// Find the nearest tsconfig.json by walking up parent directories.
    ///
    /// Returns the TsConfigId if found, or None if no tsconfig.json exists in any parent.
    pub fn find_tsconfig_id(&self, path: &Path) -> Option<TsConfigId> {
        let mut ctx = ResolutionContext::default();
        let tsconfig_id = self.find_tsconfig(path, &mut ctx).ok()??;
        Some(tsconfig_id)
    }

    /// Find the nearest dsconfig.json by walking up parent directories.
    ///
    /// Returns the DsConfigId if found, or None if no dsconfig.json exists in any parent.
    pub fn find_dsconfig(&self, path: &Path) -> Option<DsConfigId> {
        let mut ctx = ResolutionContext::default();

        // go up directories when the querying path is not a directory
        let mut current = path.to_path_buf();
        while !self.is_directory(&current, &mut ctx) {
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        // traverse parents looking for dsconfig.json
        let mut current = Some(current);
        while let Some(dir) = current {
            let dsconfig_path = dir.join("dsconfig.json");

            // check if already in registry
            if let Some(dsconfig_id) = self.program.dsconfigs.get_id_by_path(&dsconfig_path) {
                return Some(dsconfig_id);
            }

            // check if file exists
            if self.is_file(&dsconfig_path, &mut ctx) {
                // nocheckin TODO @Incomplete: should load and register dsconfig here
                return None;
            }

            current = dir.parent().map(|p| p.to_path_buf());
        }

        None
    }
}
