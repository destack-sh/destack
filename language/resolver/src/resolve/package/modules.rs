use std::path::{Path, PathBuf};

use destack_source::{FileType, PathExt};

use crate::{CachePolicy, ResolveContext, ResolveError, Resolver};

#[allow(clippy::too_many_arguments)]
impl Resolver {
    pub(crate) fn load_package_self_or_modules(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<PathBuf, ResolveError> {
        let (package_name, subpath) = Self::parse_package_specifier(specifier);
        if subpath.is_empty() {
            ctx.is_fully_specified = false;
        }

        // try to load from the package itself (also checks browser field)
        if let Some(resolved) = self.load_package_self(path, specifier, ctx)? {
            return Ok(resolved);
        }

        // try to load from node_modules
        if let Some(resolved) = self.load_modules(path, specifier, package_name, subpath, ctx)? {
            return Ok(resolved);
        }

        // abnormal relative specifier like `jest-runner-../../..`
        if specifier.contains("/../..") || specifier.contains("../../") {
            let normalized_path = Path::new(specifier).normalize_relative();
            let mut normalized_specifier = normalized_path.to_string_lossy().into_owned();
            if specifier.ends_with('/') {
                normalized_specifier += "/";
            }
            let normalized_specifier = normalized_specifier.as_str();

            // try to load package from modules
            let (package_name, subpath) = Self::parse_package_specifier(normalized_specifier);
            if package_name == ".."
                && let Some(resolved) =
                    self.load_modules(path, normalized_specifier, package_name, subpath, ctx)?
            {
                return Ok(resolved);
            }
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve a `#`-prefixed import specifier against package.json imports field.
    #[tracing::instrument(
        name = "resolver.load.package.imports",
        level = "trace",
        skip(self, ctx)
    )]
    pub(crate) fn load_modules(
        &self,
        path: &Path,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(
            ?path,
            ?specifier,
            ?package_name,
            ?subpath,
            "resolver.load.modules"
        );
        // check each module directory (node_modules)
        for module_name in &self.options.modules {
            // walk up parent directories
            let mut current = Some(path.to_path_buf());
            while let Some(current_path) = current {
                // get the module directory
                let Some(module_dir) = self.get_module_directory(&current_path, module_name, ctx)
                else {
                    current = current_path.parent().map(|p| p.to_path_buf());
                    continue;
                };

                // resolve type-conditioned package entries before runtime fallback
                if self.is_types_condition_active() {
                    if let Some(resolved) = self.resolve_modules_entry(
                        &module_dir,
                        specifier,
                        package_name,
                        subpath,
                        false,
                        ctx,
                    )? {
                        return Ok(Some(resolved));
                    }

                    // resolve declaration fallback through @types packages
                    if let Some(resolved) =
                        self.resolve_types_modules_entry(&module_dir, package_name, subpath, ctx)?
                    {
                        return Ok(Some(resolved));
                    }
                }

                // resolve runtime package entries when declaration resolution did not match
                if let Some(resolved) = self.resolve_modules_entry(
                    &module_dir,
                    specifier,
                    package_name,
                    subpath,
                    true,
                    ctx,
                )? {
                    return Ok(Some(resolved));
                }

                current = current_path.parent().map(|p| p.to_path_buf());
            }
        }
        Ok(None)
    }

    /// Resolve one specifier from one concrete modules directory.
    fn resolve_modules_entry(
        &self,
        module_directory: &Path,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        allow_runtime_fallback: bool,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // optimize node_modules lookup by checking whether the package directory exists
        if !package_name.is_empty() {
            let package_path = module_directory.normalize_with(package_name);

            // try <dir>/node_modules/package_name exports first
            if self.is_directory(&package_path, ctx) {
                if let Some(resolved) =
                    self.load_package_exports(specifier, subpath, &package_path, ctx)?
                {
                    // keep declaration compatible export targets in the type prepass
                    if allow_runtime_fallback
                        || !self.is_types_condition_active()
                        || Self::path_is_types_compatible(&resolved)
                    {
                        return Ok(Some(resolved));
                    }
                }

                // resolve explicit package types fields for root package requests
                if (subpath.is_empty() || subpath == ".")
                    && self.is_types_condition_active()
                    && let Some(package_id) =
                        self.load_package(&package_path, ctx, CachePolicy::UseCache)?
                {
                    let package = self.packages.get(package_id);
                    let package = package.read();

                    if let Some(config) = &package.manifest
                        && let Some(types_field) = config.content.types.as_deref()
                    {
                        let types_path = package_path.normalize_with(types_field);
                        if self.is_file(&types_path, ctx) && self.check_restrictions(&types_path) {
                            return self.resolve_esm_match(specifier, &types_path, ctx);
                        }
                    }
                }
            }
            // package_name is not a directory, skip unless we're looking for scope
            else {
                if !subpath.is_empty() {
                    return Ok(None);
                }

                // skip if the scope directory itself doesn't exist
                if package_name.starts_with('@')
                    && let Some(parent) = package_path.parent()
                    && !self.is_directory(parent, ctx)
                {
                    return Ok(None);
                }
            }
        }

        // skip runtime fallback in type-only prepass
        if !allow_runtime_fallback {
            return Ok(None);
        }

        // try as file or directory for all other cases
        let resolved_path = module_directory.normalize_with(specifier);

        // prefer directory contexts
        if self.options.resolve_to_context {
            return Ok(self
                .is_directory(&resolved_path, ctx)
                .then(|| resolved_path.to_path_buf()));
        }

        // load directory targets
        if self.is_directory(&resolved_path, ctx) {
            if let Some(resolved) = self.load_browser_field_or_alias(&resolved_path, ctx)? {
                return Ok(Some(resolved));
            }
            if let Some(resolved) = self.load_directory(&resolved_path, ctx)? {
                return Ok(Some(resolved));
            }
        }
        // load file targets
        else if let Some(resolved) = self.load_file(&resolved_path, ctx)? {
            return Ok(Some(resolved));
        }

        Ok(None)
    }

    /// Resolve one specifier through a matching `@types/*` package when type conditions are active.
    fn resolve_types_modules_entry(
        &self,
        module_directory: &Path,
        package_name: &str,
        subpath: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // only type-conditioned resolution may use @types fallback
        if !self.is_types_condition_active() {
            return Ok(None);
        }

        // avoid recursive fallback and invalid package inputs
        if package_name.is_empty() || package_name.starts_with("@types/") {
            return Ok(None);
        }

        // map one package name into one @types package name
        let Some(types_package_name) = Self::types_package_name_for(package_name) else {
            return Ok(None);
        };

        let types_specifier = format!("{types_package_name}{subpath}");
        let (types_package_name, types_subpath) = Self::parse_package_specifier(&types_specifier);
        self.resolve_modules_entry(
            module_directory,
            &types_specifier,
            types_package_name,
            types_subpath,
            true,
            ctx,
        )
    }

    /// Return true when one resolved path is compatible with type-conditioned resolution.
    fn path_is_types_compatible(path: &Path) -> bool {
        matches!(
            FileType::from_path(path),
            Some(
                FileType::Destack
                    | FileType::DestackDeclaration
                    | FileType::TypeScript
                    | FileType::TypeScriptXml
                    | FileType::TypeScriptDeclaration
            )
        )
    }

    /// Return true when the active conditions include the TypeScript `types` condition.
    fn is_types_condition_active(&self) -> bool {
        self.options
            .conditions
            .iter()
            .any(|condition| condition == "types")
    }

    /// Convert one bare package name into its matching `@types` package name.
    fn types_package_name_for(package_name: &str) -> Option<String> {
        // map scoped packages to DefinitelyTyped scoped naming
        if package_name.starts_with('@') {
            let scoped = package_name.strip_prefix('@')?;
            let (scope, name) = scoped.split_once('/')?;

            return Some(format!("@types/{scope}__{name}"));
        }

        Some(format!("@types/{package_name}"))
    }

    /// Get a subdirectory of the given path if it exists.
    pub(super) fn get_module_directory(
        &self,
        path: &Path,
        module_name: &str,
        ctx: &mut ResolveContext,
    ) -> Option<PathBuf> {
        // check if already in the module directory
        if path
            .components()
            .next_back()
            .is_some_and(|c| c.as_os_str() == module_name)
        {
            return Some(path.to_path_buf());
        }

        // check subdirectory
        let subdir = path.join(module_name);
        if self.is_directory(&subdir, ctx) {
            Some(subdir)
        } else {
            None
        }
    }
}
