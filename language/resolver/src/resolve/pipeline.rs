use std::path::{Path, PathBuf};

use destack_source::{PathExt, SLASH_START};

use crate::{
    Resolution, ResolveError, ResolveFrame, ResolveOrigin, ResolveRequest, ResolveRequestKind,
    Resolver,
};

#[cfg(not(target_arch = "wasm32"))]
use std::borrow::Cow;

impl Resolver {
    /// Normalize one `file://` specifier into a plain path specifier.
    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_file_protocol(specifier: &str) -> Result<Cow<'_, str>, ResolveError> {
        if specifier.starts_with("file://") {
            url::Url::parse(specifier)
                .map_err(|_| ())
                .and_then(|url| {
                    url.to_file_path().map(|path| {
                        let mut result = path.to_string_lossy().to_string();
                        if let Some(query) = url.query() {
                            result.push('?');
                            result.push_str(query);
                        }
                        if let Some(fragment) = url.fragment() {
                            result.push('#');
                            result.push_str(fragment);
                        }
                        Cow::Owned(result)
                    })
                })
                .map_err(|()| ResolveError::UnsupportedPath {
                    path: PathBuf::from(specifier),
                })
        } else {
            Ok(Cow::Borrowed(specifier))
        }
    }

    /// Resolve one parsed request from one directory.
    pub(crate) fn resolve_request(
        &self,
        origin: ResolveOrigin,
        request_directory: &Path,
        lookup_path: &Path,
        request: &ResolveRequest,
        ctx: &mut ResolveFrame,
    ) -> Result<Resolution, ResolveError> {
        // guard recursive resolution depth for the whole request
        let mut depth_guard = ctx.depth_guard()?;
        let ctx = depth_guard.frame();

        // normalize `file://` requests back into plain path specifiers
        #[cfg(not(target_arch = "wasm32"))]
        if request.path.starts_with("file://") {
            let mut raw_specifier = request.path.clone();
            if let Some(query) = &request.query {
                raw_specifier.push_str(query);
            }
            if let Some(fragment) = &request.fragment {
                raw_specifier.push_str(fragment);
            }

            let normalized = Self::resolve_file_protocol(&raw_specifier)?;
            let normalized_request = ResolveRequest::parse(normalized.as_ref());
            return self.resolve_request(
                origin,
                request_directory,
                lookup_path,
                &normalized_request,
                ctx,
            );
        }

        // try the fragment as a path suffix before treating it as metadata
        if let Some(candidate) = request.fragment_path_candidate() {
            match self.resolve_request_path(origin, request_directory, lookup_path, &candidate, ctx)
            {
                Ok(resolved) => return Ok(resolved),
                Err(error) if error.is_alternative_candidate_miss() => {}
                Err(error) => return Err(error),
            }
        }

        // resolve the path part, then restore any request level query or fragment
        let resolved = self.resolve_request_path(
            origin,
            request_directory,
            lookup_path,
            request.path.as_str(),
            ctx,
        )?;
        Ok(resolved.fill_missing_parts(request.query.clone(), request.fragment.clone()))
    }

    /// Resolve one request path without query or fragment parsing.
    pub(crate) fn resolve_request_path(
        &self,
        origin: ResolveOrigin,
        request_directory: &Path,
        lookup_path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Resolution, ResolveError> {
        // reject the degenerate empty request early
        if specifier.is_empty() {
            return Err(ResolveError::InvalidSpecifier {
                specifier: specifier.to_string(),
                message: Some("empty specifier".to_string()),
            });
        }

        // apply request rewrites before classifying the specifier
        if let Some(resolved) = self.rewrite_tsconfig_paths(origin, lookup_path, specifier, ctx)? {
            return Ok(resolved);
        }

        if let Some(resolved) =
            self.rewrite_primary_alias(origin, request_directory, lookup_path, specifier, ctx)?
        {
            return Ok(resolved);
        }

        // dispatch to the concrete resolution mode
        let result = match ResolveRequest::kind_for(specifier) {
            ResolveRequestKind::Absolute => {
                self.resolve_absolute_request(request_directory, specifier, ctx)
            }
            ResolveRequestKind::Relative => {
                self.resolve_relative_request(request_directory, specifier, ctx)
            }
            ResolveRequestKind::PackageImport => {
                self.resolve_package_import_request(request_directory, specifier, ctx)
            }
            ResolveRequestKind::Bare => {
                self.resolve_bare_request(request_directory, specifier, ctx)
            }
        };

        // try fallback aliases only for ordinary candidate misses
        result.or_else(|error| {
            if error.is_ignore() || !error.is_alternative_candidate_miss() {
                return Err(error);
            }

            self.rewrite_fallback_alias(origin, request_directory, lookup_path, specifier, ctx)
                .and_then(|value| value.ok_or(error))
        })
    }

    /// Resolve one absolute filesystem request.
    fn resolve_absolute_request(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Resolution, ResolveError> {
        // allow bare style package lookup for slash specifiers when configured
        if !self.options.prefer_relative
            && self.options.prefer_absolute
            && let Ok(resolved) = self.resolve_package_or_modules(path, specifier, ctx)
        {
            return Ok(resolved);
        }

        // resolve slash specifiers against configured project roots
        if let Some(resolved) = self.resolve_from_roots(path, specifier, ctx) {
            return Ok(resolved);
        }

        // fall back to direct filesystem probing
        let specifier_path = Path::new(specifier).to_path_buf();
        if let Some(resolved) = self.probe_path(&specifier_path, specifier, ctx)? {
            return Ok(resolved);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve one relative filesystem request.
    fn resolve_relative_request(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Resolution, ResolveError> {
        let path_with_specifier = path.normalize_with(specifier);
        let probe_specifier = if specifier == "." { "./" } else { specifier };

        // probe the normalized relative path directly
        if let Some(resolved) = self.probe_path(&path_with_specifier, probe_specifier, ctx)? {
            return Ok(resolved);
        }

        Err(ResolveError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve one `#` package import request.
    fn resolve_package_import_request(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Resolution, ResolveError> {
        self.rewrite_package_import(path, specifier, ctx)?
            .ok_or_else(|| ResolveError::NotFound {
                specifier: specifier.to_string(),
            })
    }

    /// Resolve one bare package or module request.
    fn resolve_bare_request(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Result<Resolution, ResolveError> {
        // allow relative style probing first when configured
        if self.options.prefer_relative
            && let Ok(resolved) = self.resolve_relative_request(path, specifier, ctx)
        {
            return Ok(resolved);
        }

        self.resolve_package_or_modules(path, specifier, ctx)
    }

    /// Resolve one `/` request against configured root directories.
    pub(crate) fn resolve_from_roots(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveFrame,
    ) -> Option<Resolution> {
        // skip root resolution entirely when no roots are configured
        if self.options.roots.is_empty() {
            return None;
        }

        // strip the leading slash before checking configured roots
        let relative_specifier = specifier.strip_prefix(SLASH_START)?;

        // resolve the bare slash against the current root itself
        if relative_specifier.is_empty() {
            let is_root = self.options.roots.iter().any(|root| root.as_path() == path);
            if is_root && let Ok(resolved) = self.resolve_relative_request(path, "./", ctx) {
                return Some(resolved);
            }
        } else {
            // otherwise probe each configured root directory in order
            for root in &self.options.roots {
                if let Ok(resolved) = self.resolve_relative_request(root, relative_specifier, ctx) {
                    return Some(resolved);
                }
            }
        }

        None
    }
}
