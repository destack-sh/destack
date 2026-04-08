use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileType, PathExt};

use crate::{
    CachePolicy, Resolution, ResolveContext, ResolveError, ResolveOrigin, ResolvePath,
    ResolveState, Resolver,
};

#[cfg(not(target_arch = "wasm32"))]
use pnp::Resolution as PnpResolution;

#[allow(clippy::too_many_arguments)]
impl Resolver {
    /// Normalize one abnormal bare specifier when it contains parent traversal.
    fn normalize_abnormal_package_specifier(specifier: &str) -> Option<String> {
        if !specifier.contains("/../..") && !specifier.contains("../../") {
            return None;
        }

        let normalized_path = Path::new(specifier).normalize_relative();
        let mut normalized_specifier = normalized_path.to_string_lossy().into_owned();
        if specifier.ends_with('/') {
            normalized_specifier += "/";
        }

        Some(normalized_specifier)
    }

    /// Resolve one bare package request through package self references or module directories.
    pub(crate) fn resolve_package_or_modules(
        &self,
        path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        // parse the bare specifier once for the package resolution path
        let (package_name, subpath) = parse_package_specifier(specifier);
        let state = if subpath.is_empty() {
            state.with_fully_specified(false)
        } else {
            state
        };

        // try the package itself first
        if let Some(resolved) =
            self.apply_package_self_reference(path, specifier, state.clone(), ctx)?
        {
            return Ok(resolved);
        }

        // try module directory search
        if let Some(resolved) = self.resolve_modules_from_directory(
            path,
            specifier,
            package_name,
            subpath,
            state.clone(),
            ctx,
        )? {
            return Ok(resolved);
        }

        // handle abnormal relative specifiers like `jest-runner-../../..`
        if let Some(normalized_specifier) = Self::normalize_abnormal_package_specifier(specifier) {
            let (package_name, subpath) = parse_package_specifier(&normalized_specifier);
            if package_name == ".."
                && let Some(resolved) = self.resolve_modules_from_directory(
                    path,
                    &normalized_specifier,
                    package_name,
                    subpath,
                    state,
                    ctx,
                )?
            {
                return Ok(resolved);
            }
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Search configured module directories while walking up from the issuer path.
    pub(crate) fn resolve_modules_from_directory(
        &self,
        path: &Path,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        let is_types_condition_active = self.is_types_condition_active();

        // resolve through yarn pnp before module directory lookup
        #[cfg(not(target_arch = "wasm32"))]
        if self.options.yarn_pnp
            && let Some(resolved) = self.resolve_pnp(path, specifier, state.clone(), ctx)?
        {
            return Ok(Some(resolved));
        }

        // search each configured module directory
        for module_name in &self.options.modules {
            let mut current = Some(path.to_path_buf());
            while let Some(current_path) = current {
                // resolve the concrete module directory for this ancestor
                let Some(module_directory) =
                    self.get_module_directory(&current_path, module_name, ctx)
                else {
                    current = current_path.parent().map(Path::to_path_buf);
                    continue;
                };

                // try types resolution before runtime fallback
                if is_types_condition_active {
                    if let Some(resolved) = self.resolve_modules_entry(
                        &module_directory,
                        specifier,
                        package_name,
                        subpath,
                        false,
                        state.clone(),
                        ctx,
                    )? {
                        return Ok(Some(resolved));
                    }

                    // try declaration fallback through @types packages
                    if let Some(resolved) = self.resolve_types_modules_entry(
                        &module_directory,
                        package_name,
                        subpath,
                        state.clone(),
                        ctx,
                    )? {
                        return Ok(Some(resolved));
                    }
                }

                // fall back to runtime package entries
                if let Some(resolved) = self.resolve_modules_entry(
                    &module_directory,
                    specifier,
                    package_name,
                    subpath,
                    true,
                    state.clone(),
                    ctx,
                )? {
                    return Ok(Some(resolved));
                }

                current = current_path.parent().map(Path::to_path_buf);
            }
        }

        Ok(None)
    }

    /// Resolve one bare specifier through the active Yarn PnP manifest.
    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_pnp(
        &self,
        path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        // load the active pnp manifest once for this request
        let manifest = self.read_pnp_manifest(ctx)?;

        // pnpapi is a builtin for pnp aware runtimes
        if specifier == "pnpapi" {
            return Ok(Some(Resolution::path_only(manifest.manifest_path.clone())));
        }

        // resolve_to_unqualified requires a trailing slash
        let mut issuer_path = path.to_path_buf();
        issuer_path.push("");

        // ask yarn pnp for the unqualified package target
        let resolution =
            pnp::resolve_to_unqualified_via_manifest(manifest.as_ref(), specifier, &issuer_path);
        let (pnp_path, subpath) = match resolution {
            Ok(PnpResolution::Resolved(path, subpath)) => (path, subpath),
            Ok(PnpResolution::Skipped) => return Ok(None),
            Err(error) => return Err(ResolveError::YarnPnpError { error }),
        };

        // allow package self and exports checks first
        if let Some(resolved) =
            self.apply_package_self_reference(&pnp_path, specifier, state.clone(), ctx)?
        {
            return Ok(Some(resolved));
        }

        // derive one request relative to the resolved pnp package path
        let inner_request = Self::pnp_inner_request(&pnp_path, specifier, subpath.as_deref());
        let nested_candidate = pnp_path.join(&inner_request);

        // first try directory resolution for package redirects
        if self.is_directory(&nested_candidate, ctx)
            && let Some(resolved) = self.probe_directory(&nested_candidate, state.clone(), ctx)?
        {
            return Ok(Some(resolved));
        }

        // then run regular file and directory resolution from the pnp package location
        let request = ResolvePath::parse(&inner_request);
        match self.resolve_request(
            ResolveOrigin::Directory,
            &pnp_path,
            &pnp_path,
            &request,
            state,
            ctx,
        ) {
            Ok(resolved) => Ok(Some(resolved)),
            Err(error) if error.is_alternative_candidate_miss() => Err(ResolveError::NotFound {
                specifier: specifier.to_string(),
            }),
            Err(error) => Err(error),
        }
    }

    /// Compute one inner request for a path returned by Yarn PnP.
    #[cfg(not(target_arch = "wasm32"))]
    fn pnp_inner_request(pnp_path: &Path, specifier: &str, subpath: Option<&str>) -> String {
        let pnp_path_text = pnp_path.to_string_lossy();
        let package_name_in_path = pnp_path_text
            .rsplit_once("node_modules/")
            .map(|(_, tail)| tail.strip_suffix('/').unwrap_or(tail));

        // linked package paths may not include node_modules package segments
        if package_name_in_path.is_none() {
            return match subpath {
                Some(subpath) => format!("./{subpath}"),
                None => ".".to_string(),
            };
        }

        let (first, rest) = specifier.split_once('/').unwrap_or((specifier, ""));
        let package_name = if first.starts_with('@') {
            let scope_tail = rest.split_once('/').map_or(rest, |(tail, _)| tail);
            format!("{first}/{scope_tail}")
        } else {
            first.to_string()
        };
        let inner_specifier = specifier
            .strip_prefix(package_name.as_str())
            .unwrap_or(specifier);
        format!("./{}", inner_specifier.trim_start_matches('/'))
    }

    /// Load and cache one Yarn PnP manifest for the resolver cwd.
    #[cfg(not(target_arch = "wasm32"))]
    fn read_pnp_manifest(
        &self,
        ctx: &mut ResolveContext,
    ) -> Result<Arc<pnp::Manifest>, ResolveError> {
        if let Some(manifest) = ctx.pnp_manifest() {
            return Ok(manifest);
        }

        let cwd = match self.options.cwd.as_deref() {
            Some(path) => path.to_path_buf(),
            None => std::env::current_dir().map_err(|error| ResolveError::IoError {
                path: PathBuf::from("."),
                kind: error.kind(),
            })?,
        };

        match pnp::find_pnp_manifest(&cwd) {
            Ok(Some(manifest)) => {
                let manifest = Arc::new(manifest);
                ctx.remember_pnp_manifest(manifest.clone());
                Ok(manifest)
            }
            Ok(None) => Err(ResolveError::FailedToFindYarnPnpManifest { cwd }),
            Err(error) => Err(ResolveError::YarnPnpError { error }),
        }
    }

    /// Resolve one specifier from one concrete modules directory.
    fn resolve_modules_entry(
        &self,
        module_directory: &Path,
        specifier: &str,
        package_name: &str,
        subpath: &str,
        allow_runtime_fallback: bool,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        let is_types_condition_active = self.is_types_condition_active();

        // inspect the concrete package directory when the request has a package name
        if !package_name.is_empty() {
            let package_path = module_directory.normalize_with(package_name);

            // try package exports first
            if self.is_directory(&package_path, ctx) {
                if let Some(resolved) = self.resolve_package_exports(
                    specifier,
                    subpath,
                    &package_path,
                    state.clone(),
                    ctx,
                )? {
                    // keep declaration compatible export targets in the type prepass
                    if allow_runtime_fallback
                        || !is_types_condition_active
                        || Self::path_is_types_compatible(&resolved.path)
                    {
                        return Ok(Some(resolved));
                    }
                }

                // resolve explicit package types fields for root package requests
                if (subpath.is_empty() || subpath == ".")
                    && is_types_condition_active
                    && let Some(package_id) =
                        self.read_package(&package_path, ctx, CachePolicy::UseCache)?
                    && let Some(package) = ctx.package(package_id)
                    && let Some(declaration) = &package.package_declaration
                    && let Some(types_field) = declaration.manifest.types.as_deref()
                {
                    let types_path = package_path.normalize_with(types_field);
                    if self.is_file(&types_path, ctx) && self.check_restrictions(&types_path) {
                        return self.probe_esm_target(specifier, &types_path, state.clone(), ctx);
                    }
                }
            }
            // skip missing package directories unless we are still checking a scope segment
            else {
                if !subpath.is_empty() {
                    return Ok(None);
                }

                // skip if the scope directory itself does not exist
                if package_name.starts_with('@')
                    && let Some(parent) = package_path.parent()
                    && !self.is_directory(parent, ctx)
                {
                    return Ok(None);
                }
            }
        }

        // stop here in the type only prepass
        if !allow_runtime_fallback {
            return Ok(None);
        }

        // fall back to ordinary file or directory probing
        let resolved_path = module_directory.normalize_with(specifier);

        // return the directory itself in context mode
        if self.options.resolve_to_context {
            return Ok(self
                .is_directory(&resolved_path, ctx)
                .then(|| Resolution::path_only(resolved_path.to_path_buf())));
        }

        // try directory targets
        if self.is_directory(&resolved_path, ctx) {
            if let Some(resolved) = self.apply_path(&resolved_path, state.clone(), ctx)? {
                return Ok(Some(resolved));
            }

            if let Some(resolved) = self.probe_directory(&resolved_path, state.clone(), ctx)? {
                return Ok(Some(resolved));
            }
        }
        // try file targets
        else if let Some(resolved) = self.probe_file(&resolved_path, state, ctx)? {
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
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        // only type conditioned resolution may use @types fallback
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
        let (types_package_name, types_subpath) = parse_package_specifier(&types_specifier);
        self.resolve_modules_entry(
            module_directory,
            &types_specifier,
            types_package_name,
            types_subpath,
            true,
            state,
            ctx,
        )
    }

    /// Return true when one resolved path is compatible with type conditioned resolution.
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
            .is_some_and(|component| component.as_os_str() == module_name)
        {
            return Some(path.to_path_buf());
        }

        // check subdirectory
        let subdirectory = path.join(module_name);
        if self.is_directory(&subdirectory, ctx) {
            Some(subdirectory)
        } else {
            None
        }
    }
}

/// Parse one bare package specifier into package name and subpath.
fn parse_package_specifier(specifier: &str) -> (&str, &str) {
    // find first slash
    let mut separator_index = specifier.as_bytes().iter().position(|byte| *byte == b'/');

    // scoped packages have format `@scope/package/subpath`
    if specifier.starts_with('@')
        && let Some(first_slash) = separator_index
    {
        separator_index = specifier.as_bytes()[first_slash + 1..]
            .iter()
            .position(|byte| *byte == b'/')
            .map(|offset| offset + first_slash + 1);
    }

    // split at the package boundary
    let package_name = separator_index.map_or(specifier, |index| &specifier[..index]);
    let package_subpath = separator_index.map_or("", |index| &specifier[index..]);

    (package_name, package_subpath)
}
