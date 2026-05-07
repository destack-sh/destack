use std::path::Path;

use crate::{Resolution, Resolver, ResolverContext, ResolverResult, ResolverSearch};

impl Resolver {
    /// Check if a path already ends with a known extension from our extensions list.
    fn has_known_extension(&self, path: &Path) -> bool {
        let path_str = path.as_os_str().to_string_lossy();
        self.options
            .extensions
            .iter()
            .any(|extension| path_str.ends_with(extension.as_str()))
    }

    /// Try to resolve a path as a file with optional extension probing.
    pub(crate) fn probe_file(
        &self,
        path: &Path,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        if let Some(resolved) = self.probe_file_candidate(path, ctx)? {
            Ok(Some(resolved))
        } else if !self.has_known_extension(path)
            && let Some(resolved) =
                self.probe_extensions(path, &self.options.extensions, search, ctx)?
        {
            Ok(Some(resolved))
        } else {
            Ok(None)
        }
    }

    /// Try to resolve a request path as a file.
    pub(crate) fn probe_request_path(
        &self,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        if !specifier.ends_with('/')
            && let Some(resolved) = self.probe_file(path, search.clone(), ctx)?
        {
            Ok(Some(resolved))
        } else {
            Ok(None)
        }
    }

    /// Try appending configured extensions to resolve a path.
    pub(crate) fn probe_extensions(
        &self,
        path: &Path,
        extensions: &[String],
        _search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        for extension in extensions {
            let extended_path = Self::append_extension(path, extension);
            if let Some(resolved) = self.probe_file_candidate(&extended_path, ctx)? {
                return Ok(Some(resolved));
            }
        }

        Ok(None)
    }

    /// Try to resolve a direct file candidate.
    pub(crate) fn probe_file_candidate(
        &self,
        path: &Path,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        if self.is_file(path, ctx)? && self.check_restrictions(path) {
            Ok(Some(Resolution::path_only(path.to_path_buf())))
        } else {
            Ok(None)
        }
    }
}
