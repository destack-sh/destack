use std::path::Path;

use destack_source::{PathExt, SLASH_START};

use crate::{
    Resolution, Resolver, ResolverContext, ResolverError, ResolverResult, ResolverSearch,
    ResolverSpecifier, ResolverSpecifierKind,
};

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
        let search = ResolverSearch::root();

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
        match base {
            ResolverBase::File(path) => self.validate_file_base(path, ctx)?,
            ResolverBase::Directory(path) => self.validate_directory_base(path, ctx)?,
        }

        let request = ResolverSpecifier::parse(specifier);
        let request_directory = base.request_directory()?;
        let resolved = self.resolve_request(base, request_directory, &request, search, ctx)?;
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

    /// Resolve one parsed request from one directory.
    pub(crate) fn resolve_request(
        &self,
        base: ResolverBase<'_>,
        request_directory: &Path,
        request: &ResolverSpecifier,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        // descend into one recursive search
        let search = search.descend()?;

        // try the fragment as a path suffix before treating it as metadata
        if let Some(candidate) = request.fragment_path_candidate() {
            match self.resolve_request_path(
                base,
                request_directory,
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
        let resolved =
            self.resolve_request_path(base, request_directory, request.path.as_str(), search, ctx)?;
        Ok(resolved.fill_missing_suffixes(request.query.clone(), request.fragment.clone()))
    }

    /// Resolve one request path without query or fragment parsing.
    pub(crate) fn resolve_request_path(
        &self,
        base: ResolverBase<'_>,
        request_directory: &Path,
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
        if let Some(resolved) = self.resolve_alias_table(
            base,
            request_directory,
            specifier,
            &self.aliases,
            search.clone(),
            ctx,
        )? {
            return Ok(resolved);
        }

        // apply mounted package source roots before ordinary specifier dispatch
        if let Some(path) = self.mounts.path(specifier) {
            if let Some(resolved) =
                self.probe_request_path(&path, specifier, search.clone(), ctx)?
            {
                return Ok(resolved);
            }

            return Err(ResolverError::NotFound {
                specifier: specifier.to_string(),
            });
        }

        // dispatch to the concrete resolution mode
        let result = match ResolverSpecifier::kind_for(specifier) {
            ResolverSpecifierKind::Absolute => {
                self.resolve_absolute_request(request_directory, specifier, search.clone(), ctx)
            }
            ResolverSpecifierKind::Relative => {
                self.resolve_relative_request(request_directory, specifier, search.clone(), ctx)
            }
            ResolverSpecifierKind::Hash => Err(ResolverError::NotFound {
                specifier: specifier.to_string(),
            }),
            ResolverSpecifierKind::Bare => Err(ResolverError::NotFound {
                specifier: specifier.to_string(),
            }),
        };

        result
    }

    /// Resolve one absolute filesystem request.
    fn resolve_absolute_request(
        &self,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Resolution> {
        // resolve slash specifiers against configured project roots
        if let Some(resolved) = self.resolve_from_roots(path, specifier, search.clone(), ctx)? {
            return Ok(resolved);
        }

        // probe the absolute path directly
        let specifier_path = Path::new(specifier).to_path_buf();
        if let Some(resolved) = self.probe_request_path(&specifier_path, specifier, search, ctx)? {
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
            self.probe_request_path(&path_with_specifier, probe_specifier, search, ctx)?
        {
            return Ok(resolved);
        }

        Err(ResolverError::NotFound {
            specifier: specifier.to_string(),
        })
    }

    /// Resolve one `/` request against configured root directories.
    fn resolve_from_roots(
        &self,
        _path: &Path,
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

        // only file-shaped paths are resolved from roots
        if relative_specifier.is_empty() {
            return Ok(None);
        }

        // probe each configured root directory in order
        for root in &self.options.roots {
            match self.resolve_relative_request(root, relative_specifier, search.clone(), ctx) {
                Ok(resolved) => return Ok(Some(resolved)),
                Err(error) if error.is_alternative_candidate_miss() => {}
                Err(error) => return Err(error),
            }
        }

        Ok(None)
    }
}
