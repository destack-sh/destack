use std::path::{Path, PathBuf};

use destack_source::{PackageId, PathExt, SLASH_START};
use destack_workspace::{Revision, TsConfigDeclaration};

use crate::{
    Resolution, ResolveContext, ResolveError, ResolvePath, ResolvePathKind, ResolveState, Resolver,
};

#[cfg(not(target_arch = "wasm32"))]
use std::borrow::Cow;

/// The kind of path from which resolution starts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveOrigin {
    /// Resolve relative to a concrete issuer file.
    File,
    /// Resolve relative to an issuer directory.
    Directory,
}

/// One explicit resolution query.
#[derive(Clone, Copy, Debug)]
pub struct ResolveQuery<'a> {
    /// The repository revision being resolved against.
    pub revision: Revision,
    /// The issuer kind for the query.
    pub origin: ResolveOrigin,
    /// The issuer path for the query.
    pub path: &'a Path,
    /// The raw specifier string.
    pub specifier: &'a str,
}

/// Dependency tracing gathered during one resolve.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResolveTrace {
    /// The files found while probing candidates.
    pub found_dependencies: Vec<PathBuf>,
    /// The files missed while probing candidates.
    pub missing_dependencies: Vec<PathBuf>,
}

impl Resolver {
    /// Resolve one explicit query.
    pub fn resolve(&self, query: ResolveQuery<'_>) -> Result<Resolution, ResolveError> {
        let mut ctx = ResolveContext::new(query.revision);
        let state = ResolveState::new(&self.options);

        self.resolve_query(query, state, &mut ctx)
    }

    /// Resolve a specifier from a directory within one repository revision.
    pub fn resolve_from_directory<P: AsRef<Path>>(
        &self,
        revision: Revision,
        directory: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        self.resolve(ResolveQuery {
            revision,
            origin: ResolveOrigin::Directory,
            path: directory.as_ref(),
            specifier,
        })
    }

    /// Resolve a specifier from a file within one repository revision.
    pub fn resolve_from_file<P: AsRef<Path>>(
        &self,
        revision: Revision,
        file: P,
        specifier: &str,
    ) -> Result<Resolution, ResolveError> {
        self.resolve(ResolveQuery {
            revision,
            origin: ResolveOrigin::File,
            path: file.as_ref(),
            specifier,
        })
    }

    /// Resolve a specifier from a directory within one repository revision while collecting dependency tracing.
    pub fn resolve_from_directory_with_trace<P: AsRef<Path>>(
        &self,
        revision: Revision,
        directory: P,
        specifier: &str,
        trace: &mut ResolveTrace,
    ) -> Result<Resolution, ResolveError> {
        self.resolve_with_trace(
            ResolveQuery {
                revision,
                origin: ResolveOrigin::Directory,
                path: directory.as_ref(),
                specifier,
            },
            trace,
        )
    }

    /// Resolve a specifier from a file within one repository revision while collecting dependency tracing.
    pub fn resolve_from_file_with_trace<P: AsRef<Path>>(
        &self,
        revision: Revision,
        file: P,
        specifier: &str,
        trace: &mut ResolveTrace,
    ) -> Result<Resolution, ResolveError> {
        self.resolve_with_trace(
            ResolveQuery {
                revision,
                origin: ResolveOrigin::File,
                path: file.as_ref(),
                specifier,
            },
            trace,
        )
    }

    /// Resolve a specifier while collecting dependency tracing.
    pub fn resolve_with_trace(
        &self,
        query: ResolveQuery<'_>,
        trace: &mut ResolveTrace,
    ) -> Result<Resolution, ResolveError> {
        let mut ctx = ResolveContext::with_trace(query.revision);
        let state = ResolveState::new(&self.options);

        // resolve with explicit origin semantics
        let resolution = self.resolve_query(query, state, &mut ctx);

        // append dependencies to the caller trace
        ctx.append_trace_to(trace);

        resolution
    }

    /// Perform one resolution with explicit origin semantics.
    fn resolve_query(
        &self,
        query: ResolveQuery<'_>,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        self.resolve_query_path(query.origin, query.path, query.specifier, state, ctx)
    }

    /// Perform one resolution with explicit origin semantics.
    pub(crate) fn resolve_query_path(
        &self,
        origin: ResolveOrigin,
        path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        match origin {
            ResolveOrigin::File => self.validate_file_origin(path, ctx)?,
            ResolveOrigin::Directory => self.validate_directory_origin(path, ctx)?,
        }

        // derive the request directory from the origin
        let request_directory = match origin {
            ResolveOrigin::File => path.parent().unwrap_or(path),
            ResolveOrigin::Directory => path,
        };

        let request = ResolvePath::parse(specifier);
        let resolved =
            self.resolve_request(origin, request_directory, path, &request, state, ctx)?;
        let path = self.finalize_path(&resolved.path, ctx)?;

        Ok(Resolution {
            path,
            query: resolved.query,
            fragment: resolved.fragment,
        })
    }

    /// Validate that one file origin does not point at a directory.
    pub(crate) fn validate_file_origin(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<(), ResolveError> {
        match self.path_metadata(path, ctx)? {
            Some(metadata) if metadata.is_file => Ok(()),
            Some(_) => Err(ResolveError::ExpectedFilePath {
                path: path.to_path_buf(),
            }),
            None => Ok(()),
        }
    }

    /// Validate that one directory origin does not point at a file.
    pub(crate) fn validate_directory_origin(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<(), ResolveError> {
        match self.path_metadata(path, ctx)? {
            Some(metadata) if metadata.is_directory => Ok(()),
            Some(_) => Err(ResolveError::ExpectedDirectoryPath {
                path: path.to_path_buf(),
            }),
            None => Ok(()),
        }
    }

    /// Find the nearest package.json by walking up parent directories.
    pub fn find_package(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<PackageId>, ResolveError> {
        let mut ctx = ResolveContext::new(revision);
        self.find_package_scope(path, &mut ctx)
    }

    /// Find the nearest package root directory by walking up parent directories.
    pub fn find_package_root(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<PathBuf>, ResolveError> {
        let mut ctx = ResolveContext::new(revision);
        let Some(package_id) = self.find_package_scope(path, &mut ctx)? else {
            return Ok(None);
        };

        Ok(ctx
            .package(package_id)
            .and_then(|package| package.package.path.clone()))
    }

    /// Find the effective tsconfig.json for one file path.
    pub fn find_tsconfig_for_file(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<TsConfigDeclaration>, ResolveError> {
        let mut ctx = ResolveContext::new(revision);
        self.validate_file_origin(path, &mut ctx)?;
        self.find_applicable_tsconfig(ResolveOrigin::File, path, &mut ctx)
    }

    /// Find the effective tsconfig.json for one directory path.
    pub fn find_tsconfig_for_directory(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<TsConfigDeclaration>, ResolveError> {
        let mut ctx = ResolveContext::new(revision);
        self.validate_directory_origin(path, &mut ctx)?;
        self.find_applicable_tsconfig(ResolveOrigin::Directory, path, &mut ctx)
    }

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
        request: &ResolvePath,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        // enter one recursive resolve frame
        let state = state.enter()?;

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
            let normalized_request = ResolvePath::parse(normalized.as_ref());
            return self.resolve_request(
                origin,
                request_directory,
                lookup_path,
                &normalized_request,
                state,
                ctx,
            );
        }

        // try the fragment as a path suffix before treating it as metadata
        if let Some(candidate) = request.fragment_path_candidate() {
            match self.resolve_request_path(
                origin,
                request_directory,
                lookup_path,
                &candidate,
                state.clone(),
                ctx,
            ) {
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
            state,
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
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        // reject the degenerate empty request early
        if specifier.is_empty() {
            return Err(ResolveError::InvalidSpecifier {
                specifier: specifier.to_string(),
                message: Some("empty specifier".to_string()),
            });
        }

        // apply request rewrites before classifying the specifier
        if let Some(resolved) =
            self.apply_tsconfig_paths(origin, lookup_path, specifier, state.clone(), ctx)?
        {
            return Ok(resolved);
        }

        if let Some(resolved) = self.apply_primary_alias(
            origin,
            request_directory,
            lookup_path,
            specifier,
            state.clone(),
            ctx,
        )? {
            return Ok(resolved);
        }

        // dispatch to the concrete resolution mode
        let result = match ResolvePath::kind_for(specifier) {
            ResolvePathKind::Absolute => {
                self.resolve_absolute_request(request_directory, specifier, state.clone(), ctx)
            }
            ResolvePathKind::Relative => {
                self.resolve_relative_request(request_directory, specifier, state.clone(), ctx)
            }
            ResolvePathKind::PackageImport => self.resolve_package_import_request(
                request_directory,
                specifier,
                state.clone(),
                ctx,
            ),
            ResolvePathKind::Bare => {
                self.resolve_bare_request(request_directory, specifier, state.clone(), ctx)
            }
        };

        // try fallback aliases only for ordinary candidate misses
        result.or_else(|error| {
            if error.is_ignore() || !error.is_alternative_candidate_miss() {
                return Err(error);
            }

            self.apply_fallback_alias(
                origin,
                request_directory,
                lookup_path,
                specifier,
                state,
                ctx,
            )
            .and_then(|value| value.ok_or(error))
        })
    }

    /// Resolve one absolute filesystem request.
    fn resolve_absolute_request(
        &self,
        path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        // allow bare style package lookup for slash specifiers when configured
        if !self.options.prefer_relative && self.options.prefer_absolute {
            match self.resolve_package_or_modules(path, specifier, state.clone(), ctx) {
                Ok(resolved) => return Ok(resolved),
                Err(error) if error.is_alternative_candidate_miss() => {}
                Err(error) => return Err(error),
            }
        }

        // resolve slash specifiers against configured project roots
        if let Some(resolved) = self.resolve_from_roots(path, specifier, state.clone(), ctx)? {
            return Ok(resolved);
        }

        // fall back to direct filesystem probing
        let specifier_path = Path::new(specifier).to_path_buf();
        if let Some(resolved) = self.probe_path(&specifier_path, specifier, state, ctx)? {
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
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        let path_with_specifier = path.normalize_with(specifier);
        let probe_specifier = if specifier == "." { "./" } else { specifier };

        // probe the normalized relative path directly
        if let Some(resolved) =
            self.probe_path(&path_with_specifier, probe_specifier, state, ctx)?
        {
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
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        self.apply_package_import(path, specifier, state, ctx)?
            .ok_or_else(|| ResolveError::NotFound {
                specifier: specifier.to_string(),
            })
    }

    /// Resolve one bare package or module request.
    fn resolve_bare_request(
        &self,
        path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Resolution, ResolveError> {
        // allow relative style probing first when configured
        if self.options.prefer_relative {
            match self.resolve_relative_request(path, specifier, state.clone(), ctx) {
                Ok(resolved) => return Ok(resolved),
                Err(error) if error.is_alternative_candidate_miss() => {}
                Err(error) => return Err(error),
            }
        }

        self.resolve_package_or_modules(path, specifier, state, ctx)
    }

    /// Resolve one `/` request against configured root directories.
    pub(crate) fn resolve_from_roots(
        &self,
        path: &Path,
        specifier: &str,
        state: ResolveState,
        ctx: &mut ResolveContext,
    ) -> Result<Option<Resolution>, ResolveError> {
        // skip root resolution entirely when no roots are configured
        if self.options.roots.is_empty() {
            return Ok(None);
        }

        // strip the leading slash before checking configured roots
        let Some(relative_specifier) = specifier.strip_prefix(SLASH_START) else {
            return Ok(None);
        };

        // resolve the bare slash against the current root itself
        if relative_specifier.is_empty() {
            let is_root = self.options.roots.iter().any(|root| root.as_path() == path);
            if is_root {
                match self.resolve_relative_request(path, "./", state.clone(), ctx) {
                    Ok(resolved) => return Ok(Some(resolved)),
                    Err(error) if error.is_alternative_candidate_miss() => {}
                    Err(error) => return Err(error),
                }
            }
        } else {
            // otherwise probe each configured root directory in order
            for root in &self.options.roots {
                match self.resolve_relative_request(root, relative_specifier, state.clone(), ctx) {
                    Ok(resolved) => return Ok(Some(resolved)),
                    Err(error) if error.is_alternative_candidate_miss() => {}
                    Err(error) => return Err(error),
                }
            }
        }

        Ok(None)
    }
}
