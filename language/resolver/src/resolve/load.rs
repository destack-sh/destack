use std::borrow::Cow;
use std::path::{Path, PathBuf};

use destack_source::PathExt;

use super::file::append_extension;
use crate::{EnforceExtension, ResolveContext, ResolveError, Resolver};

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
    #[tracing::instrument(name = "resolver.load.file", level = "trace", skip(self, ctx))]
    pub(crate) fn load_file(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(?path, "resolver.load.file");
        // try extension alias
        if let Some(resolved) = self.load_with_extension_alias(path, ctx)? {
            Ok(Some(resolved))
        }
        // if the path is a file, load it as its file extension format
        else if self.options.enforce_extension == EnforceExtension::Disabled
            && let Some(resolved) = self.load_alias_or_file(path, ctx)?
        {
            Ok(Some(resolved))
        }
        // try appending extensions (skip if path already has a known extension)
        else if !self.has_known_extension(path)
            && let Some(resolved) = self.load_extensions(path, &self.options.extensions, ctx)?
        {
            Ok(Some(resolved))
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Try to resolve a path as a directory via main files or index files.
    #[tracing::instrument(name = "resolver.load.directory", level = "trace", skip(self, ctx))]
    pub(crate) fn load_directory(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        tracing::trace!(?path, "resolver.load.directory");
        // check for package.json in the directory
        if let Some(package_id) = self.load_package(path, ctx)? {
            let package = self.packages.get(package_id);
            let package = package.read();
            if let Some(ref config) = package.manifest {
                if let Some(main_field) = config.content.main.as_deref() {
                    let main_field: Cow<'_, str> =
                        if main_field.starts_with("./") || main_field.starts_with("../") {
                            Cow::Borrowed(main_field)
                        } else {
                            Cow::Owned(format!("./{main_field}"))
                        };

                    let main_path = path.normalize_with(main_field.as_ref());

                    // try to load as file
                    if let Some(resolved) = self.load_file(&main_path, ctx)? {
                        return Ok(Some(resolved));
                    }

                    // try to load index file
                    if let Some(resolved) = self.load_index(&main_path, ctx)? {
                        return Ok(Some(resolved));
                    }
                }

                // allow `exports` field in `require('../directory')`
                if let Some(exports) = config.content.exports.as_ref()
                    && let Some(resolved) = self.package_exports_resolve(path, ".", exports, ctx)?
                {
                    return Ok(Some(resolved));
                }
            }
        }

        // try to load index file
        self.load_index(path, ctx)
    }

    /// Try to resolve a path as either a file or a directory.
    pub(crate) fn load_file_or_directory(
        &self,
        path: &Path,
        specifier: &str,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // special mode: resolve to context itself
        if self.options.resolve_to_context && self.is_directory(path, ctx) {
            return Ok(Some(path.to_path_buf()));
        }

        // try as file (unless specifier ends with `/`)
        if !specifier.ends_with('/')
            && let Some(resolved) = self.load_file(path, ctx)?
        {
            Ok(Some(resolved))
        }
        // try as directory
        else if self.is_directory(path, ctx)
            && let Some(resolved) = self.load_directory(path, ctx)?
        {
            Ok(Some(resolved))
        }
        // not found
        else {
            Ok(None)
        }
    }

    /// Try appending configured extensions to resolve a path.
    pub(crate) fn load_extensions(
        &self,
        path: &Path,
        extensions: &[String],
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        if ctx.is_fully_specified {
            return Ok(None);
        }
        for extension in extensions {
            let extended_path = append_extension(path, extension);
            if let Some(resolved) = self.load_alias_or_file(&extended_path, ctx)? {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Try to resolve a directory by looking for index files.
    pub(crate) fn load_index(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // try every main file
        for main_file in &self.options.main_files {
            let resolved_path = path.normalize_with(main_file);

            // try loading directly
            if self.options.enforce_extension == EnforceExtension::Disabled
                && let Some(resolved) = self.load_browser_field_or_alias(&resolved_path, ctx)?
                && self.check_restrictions(&resolved)
            {
                return Ok(Some(resolved));
            }

            // try extensions
            if let Some(resolved) =
                self.load_extensions(&resolved_path, &self.options.extensions, ctx)?
            {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Try to resolve via browser field or alias mappings.
    pub(crate) fn load_browser_field_or_alias(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // try browser field
        if let Some(package_id) = self.find_package_json(path, ctx)? {
            let package = self.packages.get(package_id);
            let package = package.read();
            if let Some(ref config) = package.manifest
                && let Some(resolved) = self.load_browser_field(path, None, config, ctx)?
            {
                return Ok(Some(resolved));
            }
        }
        // try alias
        if !self.options.alias.is_empty() {
            let alias_specifier = path.to_string_lossy();
            if let Some(resolved) =
                self.load_alias(path, &alias_specifier, &self.options.alias, ctx)?
            {
                return Ok(Some(resolved));
            }
        }
        Ok(None)
    }

    /// Try to resolve via browser field and alias, falling back to direct file check.
    pub(crate) fn load_alias_or_file(
        &self,
        path: &Path,
        ctx: &mut ResolveContext,
    ) -> Result<Option<PathBuf>, ResolveError> {
        // try browser field and alias first
        if let Some(resolved) = self.load_browser_field_or_alias(path, ctx)? {
            Ok(Some(resolved))
        }
        // try as direct file
        else if self.is_file(path, ctx) && self.check_restrictions(path) {
            Ok(Some(path.to_path_buf()))
        }
        // not found
        else {
            Ok(None)
        }
    }
}
