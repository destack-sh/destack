use std::path::{Path, PathBuf};

use destack_source::{PathExt, SLASH_START};

use crate::{
    Resolution, Resolver, ResolverContext, ResolverError, ResolverResult, ResolverSearch,
    ResolverSpecifier, ResolverSpecifierKind,
};

#[cfg(not(target_arch = "wasm32"))]
use std::borrow::Cow;

/// The source location from which one specifier is resolved.
#[derive(Clone, Copy, Debug)]
pub enum ResolverBase<'a> {
    /// Resolve relative to a concrete base file.
    File(&'a Path),
    /// Resolve relative to a base directory.
    Directory(&'a Path),
}

/// One explicit resolution query.
#[derive(Clone, Copy, Debug)]
pub struct ResolverQuery<'a> {
    /// The base from which the specifier is resolved.
    pub base: ResolverBase<'a>,
    /// The raw specifier string.
    pub specifier: &'a str,
}

impl<'a> ResolverBase<'a> {
    /// Return the base path.
    fn path(self) -> &'a Path {
        match self {
            Self::File(path) | Self::Directory(path) => path,
        }
    }

    /// Return the directory used for relative requests from this base.
    fn request_directory(self) -> ResolverResult<&'a Path> {
        match self {
            Self::File(path) => path
                .parent()
                .ok_or_else(|| ResolverError::ExpectedFilePath {
                    path: path.to_path_buf(),
                }),
            Self::Directory(path) => Ok(path),
        }
    }
}

impl Resolver {
    /// Resolve one explicit query.
    pub fn resolve(
        &self,
        ctx: &mut ResolverContext,
        query: ResolverQuery<'_>,
    ) -> ResolverResult<Resolution> {
        let search = ResolverSearch::root(&self.options);

        self.resolve_from_base(query.base, query.specifier, search, ctx)
    }

    /// Resolve a specifier from a directory within one repository revision.
    pub fn resolve_from_directory<P: AsRef<Path>>(
        &self,
        ctx: &mut ResolverContext,
        directory: P,
        specifier: &str,
    ) -> ResolverResult<Resolution> {
        self.resolve(
            ctx,
            ResolverQuery {
                base: ResolverBase::Directory(directory.as_ref()),
                specifier,
            },
        )
    }

    /// Resolve a specifier from a file within one repository revision.
    pub fn resolve_from_file<P: AsRef<Path>>(
        &self,
        ctx: &mut ResolverContext,
        file: P,
        specifier: &str,
    ) -> ResolverResult<Resolution> {
        self.resolve(
            ctx,
            ResolverQuery {
                base: ResolverBase::File(file.as_ref()),
                specifier,
            },
        )
    }

    /// Resolve one specifier from a validated base.
    pub(crate) fn resolve_from_base(
        &self,
        base: ResolverBase<'_>,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        let path = base.path();
        match base {
            ResolverBase::File(path) => self.validate_file_base(path, ctx)?,
            ResolverBase::Directory(path) => self.validate_directory_base(path, ctx)?,
        }

        let request = ResolverSpecifier::parse(specifier);
        let request_directory = base.request_directory()?;
        let resolved =
            self.resolve_request(base, request_directory, path, &request, search, ctx)?;
        let (path, query, fragment) = resolved.into_components();
        let path = self.finalize_path(&path, ctx)?;

        Ok(Resolution::new(path, query, fragment))
    }

    /// Validate that one file base does not point at a directory.
    pub(crate) fn validate_file_base(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<()> {
        match self.path_metadata(path, ctx)? {
            Some(metadata) if metadata.is_file => Ok(()),
            Some(_) => Err(ResolverError::ExpectedFilePath {
                path: path.to_path_buf(),
            }),
            None => Err(ResolverError::ExpectedFilePath {
                path: path.to_path_buf(),
            }),
        }
    }

    /// Validate that one directory base does not point at a file.
    pub(crate) fn validate_directory_base(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<()> {
        match self.path_metadata(path, ctx)? {
            Some(metadata) if metadata.is_directory => Ok(()),
            Some(_) => Err(ResolverError::ExpectedDirectoryPath {
                path: path.to_path_buf(),
            }),
            None => Err(ResolverError::ExpectedDirectoryPath {
                path: path.to_path_buf(),
            }),
        }
    }

    /// Normalize one `file://` specifier into a plain path specifier.
    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_file_protocol(specifier: &str) -> ResolverResult<Cow<'_, str>> {
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
                .map_err(|()| ResolverError::UnsupportedPath {
                    path: PathBuf::from(specifier),
                })
        } else {
            Ok(Cow::Borrowed(specifier))
        }
    }

    /// Resolve one parsed request from one directory.
    pub(crate) fn resolve_request(
        &self,
        base: ResolverBase<'_>,
        request_directory: &Path,
        lookup_path: &Path,
        request: &ResolverSpecifier,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        // descend into one recursive search
        let search = search.descend()?;

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
            let normalized_request = ResolverSpecifier::parse(normalized.as_ref());
            return self.resolve_request(
                base,
                request_directory,
                lookup_path,
                &normalized_request,
                search,
                ctx,
            );
        }

        // try the fragment as a path suffix before treating it as metadata
        if let Some(candidate) = request.fragment_path_candidate() {
            match self.resolve_request_path(
                base,
                request_directory,
                lookup_path,
                &candidate,
                search.clone(),
                ctx,
            ) {
                Ok(resolved) => return Ok(resolved),
                Err(error) if error.is_alternative_candidate_miss() => {}
                Err(error) => return Err(error),
            }
        }

        // resolve the path part, then restore any request level query or fragment
        let resolved = self.resolve_request_path(
            base,
            request_directory,
            lookup_path,
            request.path.as_str(),
            search,
            ctx,
        )?;
        Ok(resolved.fill_missing_suffixes(request.query.clone(), request.fragment.clone()))
    }

    /// Resolve one request path without query or fragment parsing.
    pub(crate) fn resolve_request_path(
        &self,
        base: ResolverBase<'_>,
        request_directory: &Path,
        lookup_path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        // reject the degenerate empty request early
        if specifier.is_empty() {
            return Err(ResolverError::InvalidSpecifier {
                specifier: specifier.to_string(),
                message: Some("empty specifier".to_string()),
            });
        }

        // resolve configured request rewrites before classifying the specifier
        if let Some(resolved) =
            self.resolve_tsconfig_paths(base, lookup_path, specifier, search.clone(), ctx)?
        {
            return Ok(resolved);
        }

        if let Some(resolved) = self.resolve_alias_table(
            base,
            request_directory,
            lookup_path,
            specifier,
            &self.aliases,
            search.clone(),
            ctx,
        )? {
            return Ok(resolved);
        }

        // dispatch to the concrete resolution mode
        let result = match ResolverSpecifier::kind_for(specifier) {
            ResolverSpecifierKind::Absolute => {
                self.resolve_absolute_request(request_directory, specifier, search.clone(), ctx)
            }
            ResolverSpecifierKind::Relative => {
                self.resolve_relative_request(request_directory, specifier, search.clone(), ctx)
            }
            ResolverSpecifierKind::PackageImport => self.resolve_package_import_request(
                request_directory,
                specifier,
                search.clone(),
                ctx,
            ),
            ResolverSpecifierKind::Bare => {
                self.resolve_bare_request(request_directory, specifier, search.clone(), ctx)
            }
        };

        // try fallback aliases only for ordinary candidate misses
        result.or_else(|error| {
            if error.is_ignore() || !error.is_alternative_candidate_miss() {
                return Err(error);
            }

            self.resolve_alias_table(
                base,
                request_directory,
                lookup_path,
                specifier,
                &self.fallback_aliases,
                search,
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
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        // allow bare style package lookup for slash specifiers when configured
        if !self.options.prefer_relative && self.options.prefer_absolute {
            match self.resolve_package_or_modules(path, specifier, search.clone(), ctx) {
                Ok(resolved) => return Ok(resolved),
                Err(error) if error.is_alternative_candidate_miss() => {}
                Err(error) => return Err(error),
            }
        }

        // resolve slash specifiers against configured project roots
        if let Some(resolved) = self.resolve_from_roots(path, specifier, search.clone(), ctx)? {
            return Ok(resolved);
        }

        // probe the absolute path directly
        let specifier_path = Path::new(specifier).to_path_buf();
        if let Some(resolved) = self.probe_path(&specifier_path, specifier, search, ctx)? {
            return Ok(resolved);
        }

        Err(ResolverError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve one relative filesystem request.
    fn resolve_relative_request(
        &self,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        let path_with_specifier = path.normalize_with(specifier);
        let probe_specifier = if specifier == "." { "./" } else { specifier };

        // probe the normalized relative path directly
        if let Some(resolved) =
            self.probe_path(&path_with_specifier, probe_specifier, search, ctx)?
        {
            return Ok(resolved);
        }

        Err(ResolverError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve one `#` package import request.
    fn resolve_package_import_request(
        &self,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        self.resolve_package_import(path, specifier, search, ctx)?
            .ok_or_else(|| ResolverError::NotFound {
                specifier: specifier.to_string(),
            })
    }

    /// Resolve one bare package or module request.
    fn resolve_bare_request(
        &self,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        // allow relative style probing first when configured
        if self.options.prefer_relative {
            match self.resolve_relative_request(path, specifier, search.clone(), ctx) {
                Ok(resolved) => return Ok(resolved),
                Err(error) if error.is_alternative_candidate_miss() => {}
                Err(error) => return Err(error),
            }
        }

        self.resolve_package_or_modules(path, specifier, search, ctx)
    }

    /// Resolve one `/` request against configured root directories.
    fn resolve_from_roots(
        &self,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
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
                match self.resolve_relative_request(path, "./", search.clone(), ctx) {
                    Ok(resolved) => return Ok(Some(resolved)),
                    Err(error) if error.is_alternative_candidate_miss() => {}
                    Err(error) => return Err(error),
                }
            }
        } else {
            // otherwise probe each configured root directory in order
            for root in &self.options.roots {
                match self.resolve_relative_request(root, relative_specifier, search.clone(), ctx) {
                    Ok(resolved) => return Ok(Some(resolved)),
                    Err(error) if error.is_alternative_candidate_miss() => {}
                    Err(error) => return Err(error),
                }
            }
        }

        Ok(None)
    }
}
