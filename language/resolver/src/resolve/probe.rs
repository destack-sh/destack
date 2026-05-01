use std::path::Path;

use destack_source::PathExt;

use crate::{
    EnforceExtension, Resolution, Resolver, ResolverBase, ResolverContext, ResolverResult,
    ResolverSearch,
};

impl Resolver {
    /// Check if a path already ends with a known extension from our extensions list.
    fn has_known_extension(&self, path: &Path) -> bool {
        let path_str = path.as_os_str().to_string_lossy();
        self.options
            .extensions
            .iter()
            .any(|ext| path_str.ends_with(ext.as_str()))
    }

    /// Try to resolve a path as a file with optional extension adding.
    pub(crate) fn probe_file(
        &self,
        path: &Path,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // try extension alias
        if let Some(resolved) = self.resolve_extension_alias(path, search.clone(), ctx)? {
            Ok(Some(resolved))
        }
        // if the path is a file, load it as its file extension format
        else if self.options.enforce_extension == EnforceExtension::Disabled
            && let Some(resolved) = self.probe_file_candidate(path, search.clone(), ctx)?
        {
            Ok(Some(resolved))
        }
        // try appending extensions (skip if path already has a known extension)
        else if !self.has_known_extension(path)
            && let Some(resolved) =
                self.probe_extensions(path, &self.options.extensions, search, ctx)?
        {
            Ok(Some(resolved))
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Try to resolve a path as a directory via main files or index files.
    pub(crate) fn probe_directory(
        &self,
        path: &Path,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // prefer package entry resolution when this directory is a package root
        if let Some(resolved) = self.resolve_package_directory(path, search.clone(), ctx)? {
            return Ok(Some(resolved));
        }

        // try to load index file
        self.probe_directory_index(path, search, ctx)
    }

    /// Try to resolve a path as either a file or a directory.
    pub(crate) fn probe_path(
        &self,
        path: &Path,
        specifier: &str,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // special mode: resolve to context itself
        if self.options.resolve_to_context && self.is_directory(path, ctx)? {
            return Ok(Some(Resolution::path_only(path.to_path_buf())));
        }

        // try as file (unless specifier ends with `/`)
        if !specifier.ends_with('/')
            && let Some(resolved) = self.probe_file(path, search.clone(), ctx)?
        {
            Ok(Some(resolved))
        }
        // try as directory
        else if self.is_directory(path, ctx)?
            && let Some(resolved) = self.probe_directory(path, search, ctx)?
        {
            Ok(Some(resolved))
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Try appending configured extensions to resolve a path.
    pub(crate) fn probe_extensions(
        &self,
        path: &Path,
        extensions: &[String],
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // fully specified requests may not add extensions
        if search.is_fully_specified() {
            return Ok(None);
        }

        // probe each configured extension in order
        for extension in extensions {
            let extended_path = Self::append_extension(path, extension);
            if let Some(resolved) =
                self.probe_file_candidate(&extended_path, search.clone(), ctx)?
            {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Try to resolve a directory by looking for index files.
    pub(crate) fn probe_directory_index(
        &self,
        path: &Path,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // try every main file
        for main_file in &self.options.main_files {
            let resolved_path = path.normalize_with(main_file);

            // try loading directly
            if self.options.enforce_extension == EnforceExtension::Disabled
                && let Some(resolved) =
                    self.resolve_path_candidate(&resolved_path, search.clone(), ctx)?
                && self.check_restrictions(resolved.path())
            {
                return Ok(Some(resolved));
            }

            // try extensions
            if let Some(resolved) = self.probe_extensions(
                &resolved_path,
                &self.options.extensions,
                search.clone(),
                ctx,
            )? {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Resolve browser field and alias mappings for one path candidate.
    pub(crate) fn resolve_path_candidate(
        &self,
        path: &Path,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // try browser field
        if let Some(package_id) = self.find_package_scope(path, ctx)? {
            let Some(package) = ctx.package(package_id) else {
                return Ok(None);
            };
            if let Some(ref declaration) = package.package_declaration
                && let Some(resolved) =
                    self.resolve_browser_field(path, None, declaration, search.clone(), ctx)?
            {
                return Ok(Some(resolved));
            }
        }

        // try alias
        if !self.options.alias.is_empty() {
            let alias_specifier = path.to_string_lossy();
            if let Some(resolved) = self.resolve_alias_table(
                ResolverBase::Directory(path),
                path,
                path,
                &alias_specifier,
                &self.aliases,
                search,
                ctx,
            )? {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Try to resolve via browser field and alias, falling back to direct file check.
    pub(crate) fn probe_file_candidate(
        &self,
        path: &Path,
        search: ResolverSearch,
        ctx: &mut ResolverContext,
    ) -> ResolverResult<Option<Resolution>> {
        // try browser field and alias first
        if let Some(resolved) = self.resolve_path_candidate(path, search, ctx)? {
            Ok(Some(resolved))
        }
        // try as direct file
        else if self.is_file(path, ctx)? && self.check_restrictions(path) {
            Ok(Some(Resolution::path_only(path.to_path_buf())))
        }
        // not found
        else {
            Ok(None)
        }
    }
}
